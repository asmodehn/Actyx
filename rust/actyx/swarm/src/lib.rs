pub mod access;
pub mod convert;
mod discovery;
pub mod metrics;
mod prune;
mod sqlite;
mod sqlite_index_store;
mod streams;
pub mod transport;
mod unixfsv1;

#[cfg(test)]
mod tests;
mod v1;
mod v2;

pub use crate::sqlite_index_store::DbPath;
pub use crate::streams::StreamAlias;
pub use crate::v1::{EventStore, Present};

use crate::prune::RetainConfig;
use crate::sqlite::{SqliteStore, SqliteStoreWrite};
use crate::sqlite_index_store::SqliteIndexStore;
use crate::streams::{OwnStreamInner, ReplicatedStreamInner, StreamMaps};
use actyxos_sdk::{
    LamportTimestamp, NodeId, Offset, OffsetMap, OffsetOrMin, Payload, StreamId, StreamNr, TagSet, Timestamp,
};
use anyhow::{Context, Result};
use ax_futures_util::{prelude::*, stream::variable::Variable};
use banyan::{
    forest::{self, BranchCache, Config as ForestConfig, CryptoConfig},
    index::Index,
    query::Query,
};
use crypto::KeyPair;
use forest::FilteredChunk;
use futures::{channel::mpsc, prelude::*};
use ipfs_embed::{
    BitswapConfig, Cid, Config as IpfsConfig, ListenerEvent, Multiaddr, NetworkConfig, PeerId, StorageConfig,
    SyncEvent, ToLibp2p,
};
use libp2p::{
    gossipsub::{GossipsubConfigBuilder, ValidationMode},
    identify::IdentifyConfig,
    multiaddr::Protocol,
    ping::PingConfig,
};
use maplit::btreemap;
use parking_lot::Mutex;
use std::{
    collections::{BTreeMap, VecDeque},
    convert::{TryFrom, TryInto},
    fmt::Debug,
    num::NonZeroU32,
    ops::RangeInclusive,
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use trees::axtrees::{AxKey, AxTrees, Sha256Digest};
use util::formats::NodeErrorContext;

#[allow(clippy::upper_case_acronyms)]
type TT = AxTrees;
type Key = AxKey;
type Event = Payload;
type Forest = banyan::forest::Forest<TT, Event, SqliteStore>;
type Transaction = banyan::forest::Transaction<TT, Event, SqliteStore, SqliteStoreWrite>;
type Link = Sha256Digest;
type Tree = banyan::tree::Tree<TT>;

pub type Block = libipld::Block<libipld::DefaultParams>;
pub type Ipfs = ipfs_embed::Ipfs<libipld::DefaultParams>;

// TODO fix stream nr
static DISCOVERY_STREAM_NR: u64 = 1;
static METRICS_STREAM_NR: u64 = 2;

#[derive(Debug, Clone, PartialEq)]
pub struct EphemeralEventsConfig {
    interval: Duration,
    streams: BTreeMap<StreamNr, RetainConfig>,
}
impl Default for EphemeralEventsConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30 * 60),
            streams: btreemap! {
                DISCOVERY_STREAM_NR.into() => RetainConfig::Events(1000),
                METRICS_STREAM_NR.into() => RetainConfig::Events(1000)
            },
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SwarmConfig {
    pub topic: String,
    pub index_store: Option<Arc<Mutex<rusqlite::Connection>>>,
    pub enable_publish: bool,
    pub enable_mdns: bool,
    pub keypair: Option<KeyPair>,
    pub psk: Option<[u8; 32]>,
    pub node_name: Option<String>,
    pub db_path: Option<PathBuf>,
    pub external_addresses: Vec<Multiaddr>,
    pub listen_addresses: Vec<Multiaddr>,
    pub bootstrap_addresses: Vec<Multiaddr>,
    pub ephemeral_event_config: EphemeralEventsConfig,
}

impl PartialEq for SwarmConfig {
    fn eq(&self, other: &Self) -> bool {
        self.topic == other.topic
            && self.enable_publish == other.enable_publish
            && self.enable_mdns == other.enable_mdns
            && self.keypair == other.keypair
            && self.psk == other.psk
            && self.node_name == other.node_name
            && self.db_path == other.db_path
            && self.external_addresses == other.external_addresses
            && self.listen_addresses == other.listen_addresses
            && self.bootstrap_addresses == other.bootstrap_addresses
            && self.ephemeral_event_config == other.ephemeral_event_config
    }
}

/// Stream manager.
#[derive(Clone)]
pub struct BanyanStore(Arc<BanyanStoreInner>);

#[derive(Clone, Debug, Default)]
struct SwarmOffsets {
    /// Currently validated OffsetMap
    present: OffsetMap,
    /// OffsetMap describing the replication target. Currently this is driven via `highest_seen`,
    /// but should eventually be fed by the partial replication mechanism.
    replication_target: OffsetMap,
}

/// internal state of the stream manager
struct BanyanStoreInner {
    maps: Mutex<StreamMaps>,
    gossip_v2: v2::GossipV2,
    forest: Forest,
    ipfs: Ipfs,
    node_id: NodeId,
    index_store: Mutex<SqliteIndexStore>,
    /// maximum ingested offset and highest seen for each stream
    offsets: Variable<SwarmOffsets>,
    /// lamport timestamp for publishing to internal streams
    lamport: Variable<LamportTimestamp>,
    /// tasks of the stream manager.
    tasks: Mutex<Vec<tokio::task::JoinHandle<()>>>,
}

impl Drop for BanyanStoreInner {
    fn drop(&mut self) {
        for task in self.tasks.lock().drain(..) {
            task.abort();
        }
    }
}

impl BanyanStore {
    /// Creates a new [`BanyanStore`] from a [`SwarmConfig`].
    pub async fn new(cfg: SwarmConfig) -> Result<Self> {
        tracing::debug!("client_from_config({:?})", cfg);
        if cfg.enable_publish {
            tracing::debug!("Publishing is allowed to pubsub");
        } else {
            tracing::debug!("Publishing is disabled to pubsub");
        }
        tracing::debug!("Start listening on topic '{}'", &cfg.topic);

        let keypair = cfg.keypair.unwrap_or_else(KeyPair::generate);
        let node_id = keypair.into();
        let node_key: ipfs_embed::Keypair = keypair.into();
        let public = node_key.to_public();
        let node_name = cfg
            .node_name
            .unwrap_or_else(|| names::Generator::with_naming(names::Name::Numbered).next().unwrap());

        let ipfs = Ipfs::new(IpfsConfig {
            network: NetworkConfig {
                node_key,
                node_name,
                psk: cfg.psk,
                quic: Default::default(),
                mdns: if cfg.enable_mdns {
                    Some(Default::default())
                } else {
                    None
                },
                kad: None,
                dns: None,
                ping: Some(
                    PingConfig::new()
                        .with_keep_alive(true)
                        .with_max_failures(NonZeroU32::new(2).unwrap()),
                ),
                identify: Some(IdentifyConfig::new("/actyx/2.0.0".to_string(), public)),
                gossipsub: Some(
                    GossipsubConfigBuilder::default()
                        .validation_mode(ValidationMode::Permissive)
                        .build()
                        .expect("valid gossipsub config"),
                ),
                broadcast: Some(Default::default()),
                bitswap: Some(BitswapConfig {
                    request_timeout: Duration::from_secs(10),
                    connection_keep_alive: Duration::from_secs(10),
                }),
            },
            storage: StorageConfig {
                path: cfg.db_path,
                cache_size_blocks: u64::MAX,
                cache_size_bytes: 1024 * 1024 * 1024 * 4,
                gc_interval: Duration::from_secs(10),
                gc_min_blocks: 1000,
                gc_target_duration: Duration::from_millis(10),
            },
        })
        .await?;

        let index_store = if let Some(conn) = cfg.index_store {
            SqliteIndexStore::from_conn(conn)?
        } else {
            SqliteIndexStore::open(DbPath::Memory)?
        };
        let forest = Forest::new(
            SqliteStore::wrap(ipfs.clone()),
            BranchCache::<TT>::new(64 << 20),
            CryptoConfig::default(),
            // TODO: add default implementation.
            ForestConfig::debug(),
        );
        let gossip_v2 = v2::GossipV2::new(ipfs.clone(), node_id, cfg.topic.clone());
        let banyan = Self(Arc::new(BanyanStoreInner {
            maps: Mutex::new(StreamMaps::default()),
            index_store: Mutex::new(index_store),
            node_id,
            ipfs,
            gossip_v2,
            forest,
            lamport: Default::default(),
            offsets: Default::default(),
            tasks: Default::default(),
        }));
        banyan.load_known_streams()?;
        banyan.spawn_task(
            "v2_gossip_ingest",
            banyan.0.gossip_v2.ingest(banyan.clone(), cfg.topic.clone())?,
        );
        banyan.spawn_task("compaction", banyan.clone().compaction_loop(Duration::from_secs(60)));
        banyan.spawn_task("v1_gossip_publish", banyan.clone().v1_gossip_publish(cfg.topic.clone()));
        banyan.spawn_task("v1_gossip_ingest", banyan.clone().v1_gossip_ingest(cfg.topic));
        banyan.spawn_task("discovery_ingest", crate::discovery::discovery_ingest(banyan.clone()));
        banyan.spawn_task(
            "discovery_publish",
            crate::discovery::discovery_publish(
                banyan.clone(),
                DISCOVERY_STREAM_NR.into(),
                cfg.external_addresses.iter().cloned().collect(),
            )?,
        );
        banyan.spawn_task(
            "metrics",
            crate::metrics::metrics(banyan.clone(), METRICS_STREAM_NR.into(), Duration::from_secs(30))?,
        );
        banyan.spawn_task(
            "prune_events",
            crate::prune::prune(banyan.clone(), cfg.ephemeral_event_config),
        );

        let ipfs = banyan.ipfs();
        for addr in cfg.listen_addresses {
            if let Some(ListenerEvent::NewListenAddr(bound_addr)) = ipfs.listen_on(addr.clone())?.next().await {
                tracing::info!(target: "SWARM_SERVICES_BOUND", "Swarm Services bound to {}.", bound_addr);
            } else {
                let port = addr
                    .iter()
                    .find_map(|x| match x {
                        Protocol::Tcp(p) => Some(p),
                        Protocol::Udp(p) => Some(p),
                        _ => None,
                    })
                    .unwrap_or_default();
                return Err(anyhow::anyhow!("failed to bind address")).with_context(|| NodeErrorContext::BindFailed {
                    port,
                    component: "Swarm".into(),
                });
            }
        }
        for addr in cfg.external_addresses {
            ipfs.add_external_address(addr);
        }
        for mut addr in cfg.bootstrap_addresses {
            let addr_orig = addr.clone();
            if let Some(Protocol::P2p(peer_id)) = addr.pop() {
                let peer_id =
                    PeerId::from_multihash(peer_id).map_err(|_| anyhow::anyhow!("invalid bootstrap peer id"))?;
                if peer_id == ipfs.local_peer_id() {
                    tracing::warn!("Not dialing configured bootstrap node {} as it's myself", addr_orig);
                } else {
                    ipfs.dial_address(&peer_id, addr)
                        .with_context(|| format!("Dialing bootstrap node {}", addr_orig))?;
                }
            } else {
                return Err(anyhow::anyhow!("invalid bootstrap address"));
            }
        }

        Ok(banyan)
    }

    /// Creates a new [`BanyanStore`] for testing.
    pub async fn test(node_name: &str) -> Result<Self> {
        Self::new(SwarmConfig {
            topic: "topic".into(),
            enable_publish: true,
            enable_mdns: true,
            node_name: Some(node_name.into()),
            listen_addresses: vec!["/ip4/127.0.0.1/tcp/0".parse().unwrap()],
            ..Default::default()
        })
        .await
    }

    fn load_known_streams(&self) -> Result<()> {
        let known_streams = self.0.index_store.lock().get_observed_streams()?;
        for stream_id in known_streams {
            tracing::debug!("Trying to load tree for {}", stream_id);
            if let Some(cid) = self.ipfs().resolve(StreamAlias::from(stream_id))? {
                let root = cid.try_into()?;
                let tree = self.0.forest.load_tree(root)?;
                self.update_present(stream_id, tree.offset())?;
                if stream_id.node_id() == self.node_id() {
                    self.get_or_create_own_stream(stream_id.stream_nr()).set_latest(tree);
                } else {
                    self.get_or_create_replicated_stream(stream_id).set_latest(tree);
                }
            } else {
                tracing::warn!("No alias found for StreamId \"{}\"", stream_id);
            }
        }

        Ok(())
    }

    /// Returns the [`NodeId`].
    pub fn node_id(&self) -> NodeId {
        self.0.node_id
    }

    /// Returns the underlying [`Ipfs`].
    pub fn ipfs(&self) -> &Ipfs {
        &self.0.ipfs
    }

    pub fn cat(&self, cid: Cid, path: VecDeque<String>) -> impl Stream<Item = Result<Vec<u8>>> + Send {
        unixfsv1::UnixfsStream::new(unixfsv1::UnixfsDecoder::new(self.ipfs().clone(), cid, path))
    }

    /// Append events to a stream, publishing the new data.
    pub async fn append(&self, stream_nr: StreamNr, events: Vec<(TagSet, Event)>) -> Result<Option<Link>> {
        tracing::debug!("publishing {} events on stream {}", events.len(), stream_nr);
        let lamport = self.0.index_store.lock().increment_lamport()?;
        let timestamp = Timestamp::now();
        let events = events
            .into_iter()
            .map(move |(tags, event)| (Key::new(tags, lamport, timestamp), event));
        self.transform_stream(stream_nr, |txn, tree| txn.extend_unpacked(tree, events))
            .await
    }

    /// Spawns a new task that will be shutdown when [`BanyanStore`] is dropped.
    pub fn spawn_task(&self, name: &'static str, task: impl Future<Output = ()> + Send + 'static) {
        tracing::debug!("Spawning task '{}'!", name);
        let handle =
            tokio::spawn(task.map(move |_| tracing::error!("Fatal: Task '{}' unexpectedly terminated!", name)));
        self.0.tasks.lock().push(handle);
    }

    /// Returns a [`Stream`] of known [`StreamId`].
    pub fn stream_known_streams(&self) -> impl Stream<Item = StreamId> + Send {
        let mut state = self.0.maps.lock();
        let (s, r) = mpsc::unbounded();
        for stream_id in state.current_stream_ids(self.0.node_id) {
            let _ = s.unbounded_send(stream_id);
        }
        state.known_streams.push(s);
        r
    }

    /// Returns a [`Stream`] of events in causal order filtered with a [`Query`].
    pub fn stream_filtered_stream_ordered<Q: Query<TT> + Clone + 'static>(
        &self,
        query: Q,
    ) -> impl Stream<Item = Result<(u64, Key, Event)>> {
        let this = self.clone();
        self.stream_known_streams()
            .map(move |stream_id| this.stream_filtered_chunked(stream_id, 0..=u64::max_value(), query.clone()))
            .merge_unordered()
            .map_ok(|chunk| stream::iter(chunk.data).map(Ok))
            .try_flatten()
    }

    pub fn stream_filtered_chunked<Q: Query<TT> + Clone + 'static>(
        &self,
        stream_id: StreamId,
        range: RangeInclusive<u64>,
        query: Q,
    ) -> impl Stream<Item = Result<FilteredChunk<TT, Event, ()>>> {
        tracing::debug!("stream_filtered_chunked {}", stream_id);
        let (trees, forest) = self.tree_stream(stream_id);
        forest.stream_trees_chunked(query, trees, range, &|_| {})
    }

    pub fn stream_filtered_chunked_reverse<Q: Query<TT> + Clone + 'static>(
        &self,
        stream_id: StreamId,
        range: RangeInclusive<u64>,
        query: Q,
    ) -> impl Stream<Item = Result<FilteredChunk<TT, Event, ()>>> {
        let (trees, forest) = self.tree_stream(stream_id);
        forest.stream_trees_chunked_reverse(query, trees, range, &|_| {})
    }

    /// careful ingestion - basically just call sync_one on each new ingested root
    async fn careful_ingestion(self, stream_id: StreamId, state: Arc<ReplicatedStreamInner>) {
        state
            .incoming_root_stream()
            .switch_map(move |root| self.clone().sync_one(stream_id, root).into_stream())
            .for_each(|_| future::ready(()))
            .await
    }

    fn get_or_create_own_stream(&self, stream_nr: StreamNr) -> Arc<OwnStreamInner> {
        let mut maps = self.0.maps.lock();
        maps.own_streams.get(&stream_nr).cloned().unwrap_or_else(|| {
            tracing::debug!("creating new own stream {}", stream_nr);
            let forest = self.0.forest.clone();
            let stream_id = self.node_id().stream(stream_nr);
            // TODO: Maybe this fn should be fallible
            let _ = self.0.index_store.lock().add_stream(stream_id);
            tracing::debug!("publish new stream_id {}", stream_id);
            maps.publish_new_stream_id(stream_id);
            let stream = Arc::new(OwnStreamInner::new(forest));
            maps.own_streams.insert(stream_nr, stream.clone());
            stream
        })
    }

    fn get_or_create_replicated_stream(&self, stream_id: StreamId) -> Arc<ReplicatedStreamInner> {
        debug_assert!(self.node_id() != stream_id.node_id());
        let mut maps = self.0.maps.lock();
        let _ = self.0.index_store.lock().add_stream(stream_id);
        let node_id = stream_id.node_id();
        let stream_nr = stream_id.stream_nr();
        let remote_node = maps.get_or_create_remote_node(node_id);
        if let Some(state) = remote_node.streams.get(&stream_nr).cloned() {
            state
        } else {
            tracing::debug!("creating new replicated stream {}", stream_id);
            let forest = self.0.forest.clone();
            let state = Arc::new(ReplicatedStreamInner::new(forest));
            self.spawn_task(
                "careful_ingestion",
                self.clone().careful_ingestion(stream_id, state.clone()),
            );
            remote_node.streams.insert(stream_nr, state.clone());
            tracing::debug!("publish new stream_id {}", stream_id);
            maps.publish_new_stream_id(stream_id);
            state
        }
    }

    fn transform_stream(
        &self,
        stream_nr: StreamNr,
        f: impl FnOnce(&Transaction, &Tree) -> Result<Tree> + Send,
    ) -> impl Future<Output = Result<Option<Link>>> {
        let this = self.clone();
        async move {
            let stream = this.get_or_create_own_stream(stream_nr);
            let lock = stream.sequencer.lock().await;
            let writer = this.0.forest.store().write()?;
            tracing::debug!("starting write transaction on stream {}", stream_nr);
            let txn = Transaction::new(stream.forest.clone(), writer);
            let curr = stream.latest();
            let tree = f(&txn, &curr)?;
            // root of the new tree
            let root: Option<Link> = tree.link();
            // check for change
            if root != curr.link() {
                let cid: Option<Cid> = root.map(Into::into);
                let stream_id = this.node_id().stream(stream_nr);
                tracing::debug!(
                    "updating alias for stream {} to {:?}",
                    stream_nr,
                    cid.map(|x: Cid| x.to_string())
                );
                // update the permanent alias
                this.ipfs().alias(StreamAlias::from(stream_id), cid.as_ref())?;
                // update present for stream
                this.update_present(stream_id, tree.offset())?;
                // update latest
                tracing::debug!("set_latest! {}", tree);
                stream.set_latest(tree);
                let blocks = txn.into_writer().into_written();
                // publish new blocks and root
                this.0.gossip_v2.publish(stream_nr, root.expect("not None"), blocks)?;
            }
            tracing::debug!("ended write transaction on stream {}", stream_nr);
            drop(lock);
            Ok(root)
        }
    }

    fn update_root(&self, stream_id: StreamId, root: Link) {
        if stream_id.node_id() != self.node_id() {
            tracing::debug!("update_root {} {}", stream_id, root);
            self.get_or_create_replicated_stream(stream_id).set_incoming(root);
        }
    }

    async fn compaction_loop(self, interval: Duration) {
        loop {
            if let Err(err) = self.compact_once().await {
                tracing::error!("{}", err);
            }
            tokio::time::sleep(interval).await;
        }
    }

    async fn compact_once(&self) -> Result<()> {
        let stream_ids = self.0.maps.lock().own_streams.keys().cloned().collect::<Vec<_>>();
        for stream_id in stream_ids {
            tracing::debug!("compacting stream {}", stream_id);
            self.pack(stream_id).await?;
        }
        Ok(())
    }

    fn pack(&self, stream_nr: StreamNr) -> impl Future<Output = Result<Option<Link>>> {
        tracing::debug!("packing stream {}", stream_nr);
        self.transform_stream(stream_nr, |txn, tree| txn.pack(tree))
    }

    /// attempt to sync one stream to a new root.
    ///
    /// this future may be interrupted at any time when an even newer root comes along.
    async fn sync_one(self, stream_id: StreamId, root: Link) -> Result<()> {
        let node_name = self.ipfs().local_node_name();
        // tokio::time::delay_for(Duration::from_millis(10)).await;
        tracing::debug!("starting to sync {} to {}", stream_id, root);
        let cid = Cid::from(root);
        let ipfs = &self.0.ipfs;
        let stream = self.get_or_create_replicated_stream(stream_id);
        let validated_lamport = stream.validated().last_lamport();
        // temporarily pin the new root
        tracing::debug!("assigning temp pin to {}", root);
        let temp_pin = ipfs.create_temp_pin()?;
        ipfs.temp_pin(&temp_pin, &cid)?;
        // attempt to sync. This may take a while and is likely to be interrupted
        tracing::debug!("starting to sync {}", root);
        // create the sync stream, and log progress. Add an additional element.
        let mut sync = ipfs.sync(&cid, ipfs.peers());
        // during the sync, try to load the tree asap and abort in case it is not good
        let mut tree: Option<Tree> = None;
        let mut n: usize = 0;
        while let Some(event) = sync.next().await {
            match event {
                SyncEvent::Progress { missing } => {
                    tracing::debug!("{} sync_one: {}/{}", node_name, n, n + missing);
                    n += 1;
                }
                SyncEvent::Complete(Err(err)) => {
                    tracing::debug!("{} {}", node_name, err);
                    return Err(err);
                }
                SyncEvent::Complete(Ok(())) => {}
            }
            if tree.is_none() {
                // load the tree as soon as possible. If this fails, bail out.
                let temp = stream.forest.load_tree(root)?;
                // check that the tree is better than the one we have, otherwise bail out
                anyhow::ensure!(temp.last_lamport() > validated_lamport);
                // get the offset
                let offset = temp.offset();
                // update present.
                let _ = self.update_highest_seen(stream_id, offset);
                tree = Some(temp);
            }
        }
        let tree = tree.ok_or_else(|| anyhow::anyhow!("unable to load tree"))?;

        // if we get here, we already know that the new tree is better than its predecessor
        tracing::debug!("completed sync of {}", root);
        // once sync is successful, permanently move the alias
        tracing::debug!("updating alias {}", root);
        // assign the new root as validated
        ipfs.alias(&StreamAlias::from(stream_id), Some(&cid))?;
        self.0.index_store.lock().received_lamport(tree.last_lamport().into())?;
        tracing::debug!("sync_one complete {} => {}", stream_id, tree.offset());
        let offset = tree.offset();
        stream.set_latest(tree);
        // update present. This can fail if stream_id is not a source_id.
        let _ = self.update_present(stream_id, offset);
        // done
        Ok(())
    }

    fn has_stream(&self, stream_id: StreamId) -> bool {
        let me = stream_id.node_id() == self.node_id();
        let maps = self.0.maps.lock();
        if me {
            maps.own_streams.contains_key(&stream_id.stream_nr())
        } else {
            maps.remote_nodes
                .get(&stream_id.node_id())
                .map(|node| node.streams.contains_key(&stream_id.stream_nr()))
                .unwrap_or_default()
        }
    }

    /// stream of latest updates from either gossip (for replicated streams) or internal updates
    ///
    /// note that this does not include event updates
    fn latest_stream(&self, stream_id: StreamId) -> impl Stream<Item = (LamportTimestamp, Offset)> {
        if stream_id.node_id() == self.node_id() {
            let stream = self.get_or_create_own_stream(stream_id.stream_nr());
            self.0
                .lamport
                .new_observer()
                .filter_map(move |lamport| future::ready(stream.offset().map(|offset| (lamport, offset))))
                .left_stream()
        } else {
            self.get_or_create_replicated_stream(stream_id)
                .latest_seen
                .new_observer()
                .filter_map(future::ready)
                .right_stream()
        }
    }

    /// Get a stream of trees for a given stream id
    fn tree_stream(&self, stream_id: StreamId) -> (impl Stream<Item = Tree>, Forest) {
        let me = stream_id.node_id() == self.node_id();
        if me {
            let stream_nr = stream_id.stream_nr();
            let stream = self.get_or_create_own_stream(stream_nr);
            (stream.tree_stream(), stream.forest.clone())
        } else {
            let stream = self.get_or_create_replicated_stream(stream_id);
            (stream.tree_stream(), stream.forest.clone())
        }
    }
}

trait AxTreeExt {
    fn last_lamport(&self) -> LamportTimestamp;
    fn offset(&self) -> OffsetOrMin;
}

impl AxTreeExt for Tree {
    fn last_lamport(&self) -> LamportTimestamp {
        match self.as_index_ref() {
            Some(Index::Branch(branch)) => branch.summaries.lamport_range().max,
            Some(Index::Leaf(leaf)) => leaf.keys.lamport_range().max,
            None => Default::default(),
        }
    }
    fn offset(&self) -> OffsetOrMin {
        match self.count() {
            0 => OffsetOrMin::MIN,
            x => match u32::try_from(x) {
                Ok(fits) => (fits - 1).into(),
                Err(e) => {
                    tracing::error!("Tree's count ({}) is too big to fit into an offset ({})", x, e);
                    OffsetOrMin::MAX
                }
            },
        }
    }
}
