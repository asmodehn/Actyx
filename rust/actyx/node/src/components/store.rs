use super::{Component, ComponentRequest};
use crate::{node_settings::Settings, BindTo};
use actyxos_sdk::NodeId;
use anyhow::Result;
use api::NodeInfo;
use crossbeam::channel::{Receiver, Sender};
use crypto::KeyStoreRef;
use parking_lot::Mutex;
use std::{convert::TryInto, path::PathBuf, sync::Arc};
use swarm::{BanyanStore, SwarmConfig};
use tokio::sync::oneshot;
use tracing::*;
use util::formats::NodeCycleCount;

pub(crate) enum StoreRequest {
    GetSwarmState {
        tx: oneshot::Sender<Result<serde_json::Value>>,
    },
}
pub(crate) type StoreTx = Sender<ComponentRequest<StoreRequest>>;

impl Component<StoreRequest, SwarmConfig> for Store {
    fn get_type(&self) -> &'static str {
        "Swarm"
    }
    fn get_rx(&self) -> &Receiver<ComponentRequest<StoreRequest>> {
        &self.rx
    }
    fn handle_request(&mut self, req: StoreRequest) -> Result<()> {
        match req {
            StoreRequest::GetSwarmState { tx } => {
                if let Some(InternalStoreState { rt: _, store }) = self.state.as_ref() {
                    let peer_id = store.ipfs().local_peer_id().to_string();
                    let listen_addrs: Vec<_> = store
                        .ipfs()
                        .listeners()
                        .into_iter()
                        .map(|addr| addr.to_string())
                        .collect();
                    let external_addrs: Vec<_> = store
                        .ipfs()
                        .external_addresses()
                        .into_iter()
                        .map(|rec| rec.addr.to_string())
                        .collect();
                    let peers: Vec<_> = store
                        .ipfs()
                        .connections()
                        .into_iter()
                        .map(|(peer, addr)| (peer.to_string(), addr.to_string()))
                        .collect();
                    let _ = tx.send(Ok(serde_json::json!({
                        "peer_id": peer_id,
                        "listen_addrs": listen_addrs,
                        "external_addrs": external_addrs,
                        "peers": peers,
                    })));
                } else {
                    let _ = tx.send(Err(anyhow::anyhow!("Store not running")));
                }
            }
        }
        Ok(())
    }
    fn set_up(&mut self, settings: SwarmConfig) -> bool {
        self.store_config = Some(settings);
        true
    }
    fn start(&mut self, _err_notifier: Sender<anyhow::Error>) -> Result<()> {
        debug_assert!(self.state.is_none());
        if let Some(cfg) = self.store_config.clone() {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(self.number_of_threads.unwrap_or(2))
                .enable_all()
                .build()?;
            let bind_to = self.bind_to.clone();
            let node_info = NodeInfo::new(self.node_id, self.keystore.clone(), self.node_cycle_count);
            // client creation is setting up some tokio timers and therefore
            // needs to be called with a tokio runtime
            let store = rt.block_on(async move {
                let store = BanyanStore::new(cfg).await?;

                store.spawn_task(
                    "api",
                    api::run(node_info, store.clone(), bind_to.clone().api.into_iter()),
                );
                Ok::<BanyanStore, anyhow::Error>(store)
            })?;

            self.state = Some(InternalStoreState { rt, store });
            Ok(())
        } else {
            anyhow::bail!("no config")
        }
    }
    fn stop(&mut self) -> Result<()> {
        if let Some(InternalStoreState { rt, .. }) = self.state.take() {
            debug!("Stopping the store");
            drop(rt);
        }
        Ok(())
    }
    fn extract_settings(&self, s: Settings) -> Result<SwarmConfig> {
        let keypair = self
            .keystore
            .read()
            .get_pair(self.node_id.into())
            .ok_or_else(|| anyhow::anyhow!("No KeyPair available for KeyId {}", self.node_id))?;
        let psk: [u8; 32] = base64::decode(&s.swarm.swarm_key)?
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid psk"))?;
        let topic = s.swarm.topic.replace('/', "_");
        let db_path = self.working_dir.join(format!("{}.sqlite", topic));
        let config = SwarmConfig {
            topic,
            index_store: Some(self.db.clone()),
            enable_publish: !s.api.events.read_only,
            enable_mdns: true,
            keypair: Some(keypair),
            psk: Some(psk),
            node_name: Some(s.admin.display_name),
            db_path: Some(db_path),
            external_addresses: s
                .swarm
                .announce_addresses
                .iter()
                .map(|s| s.parse())
                .collect::<Result<_, libp2p::multiaddr::Error>>()?,
            listen_addresses: self.bind_to.swarm.clone().to_multiaddrs().collect(),
            bootstrap_addresses: s
                .swarm
                .bootstrap_nodes
                .iter()
                .map(|s| s.parse())
                .collect::<Result<_, libp2p::multiaddr::Error>>()?,
            ephemeral_event_config: Default::default(),
        };
        Ok(config)
    }
}
struct InternalStoreState {
    rt: tokio::runtime::Runtime,
    store: BanyanStore,
}
/// Struct wrapping the store service and handling its lifecycle.
pub(crate) struct Store {
    rx: Receiver<ComponentRequest<StoreRequest>>,
    state: Option<InternalStoreState>,
    store_config: Option<SwarmConfig>,
    working_dir: PathBuf,
    bind_to: BindTo,
    keystore: KeyStoreRef,
    node_id: NodeId,
    db: Arc<Mutex<rusqlite::Connection>>,
    number_of_threads: Option<usize>,
    node_cycle_count: NodeCycleCount,
}

impl Store {
    pub fn new(
        rx: Receiver<ComponentRequest<StoreRequest>>,
        working_dir: PathBuf,
        bind_to: BindTo,
        keystore: KeyStoreRef,
        node_id: NodeId,
        db: Arc<Mutex<rusqlite::Connection>>,
        node_cycle_count: NodeCycleCount,
    ) -> anyhow::Result<Self> {
        std::fs::create_dir_all(working_dir.clone())?;
        Ok(Self {
            rx,
            state: None,
            store_config: None,
            working_dir,
            bind_to,
            keystore,
            node_id,
            db,
            number_of_threads: None,
            node_cycle_count,
        })
    }
}
