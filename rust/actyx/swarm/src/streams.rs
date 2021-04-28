use crate::{AxTreeExt, Forest, Link, Tree};
use actyxos_sdk::{LamportTimestamp, NodeId, Offset, StreamId, StreamNr};
use ax_futures_util::stream::variable::{self, Variable};
use fnv::FnvHashMap;
use futures::{
    future,
    stream::{Stream, StreamExt},
};
use std::convert::{TryFrom, TryInto};
use std::sync::Arc;
use trees::RootMapEntry;

const PREFIX: u8 = b'S';

/// Helper to store a stream id as an alias name in sqlite
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StreamAlias([u8; 41]);

impl AsRef<[u8]> for StreamAlias {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl TryFrom<&[u8]> for StreamAlias {
    type Error = anyhow::Error;

    fn try_from(value: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(value.len() == 41, "StreamAlias must be 41 bytes");
        anyhow::ensure!(value[0] == PREFIX, "Prefix must be the letter S");
        let node_id = NodeId::from_bytes(&value[1..33])?;
        let stream_nr = u64::from_be_bytes(value[33..41].try_into()?).try_into()?;
        Ok(node_id.stream(stream_nr).into())
    }
}

impl From<StreamId> for StreamAlias {
    fn from(value: StreamId) -> Self {
        let mut result = [0; 41];
        result[0] = PREFIX;
        result[1..33].copy_from_slice(value.node_id().as_ref());
        result[33..41].copy_from_slice(&u64::from(value.stream_nr()).to_be_bytes());
        StreamAlias(result)
    }
}

impl TryFrom<StreamAlias> for StreamId {
    type Error = anyhow::Error;

    fn try_from(value: StreamAlias) -> anyhow::Result<Self> {
        let node_id = NodeId::from_bytes(&value.0[1..33])?;
        let stream_nr = u64::from_be_bytes(value.0[33..41].try_into()?).try_into()?;
        Ok(node_id.stream(stream_nr))
    }
}

/// Data for a single own stream, mutable state + constant data
#[derive(Debug)]
pub struct OwnStreamInner {
    pub forest: Forest,
    pub sequencer: tokio::sync::Mutex<()>,
    pub tree: Variable<Tree>,
    pub latest_seen: Variable<Option<(LamportTimestamp, Offset)>>,
}

impl OwnStreamInner {
    pub fn new(forest: Forest) -> Self {
        Self {
            forest,
            sequencer: tokio::sync::Mutex::new(()),
            tree: Variable::default(),
            latest_seen: Variable::default(),
        }
    }

    pub fn latest(&self) -> Tree {
        self.tree.get_cloned()
    }

    pub fn offset(&self) -> Option<Offset> {
        let offset_or_min = self.tree.project(|tree| tree.offset());
        Offset::from_offset_or_min(offset_or_min)
    }

    pub fn set_latest(&self, value: Tree) {
        self.tree.set(value)
    }

    pub fn tree_stream(&self) -> variable::Observer<Tree> {
        self.tree.new_observer()
    }
}

#[derive(Debug, Default)]
pub struct RemoteNodeInner {
    pub last_seen: Variable<(LamportTimestamp, Offset)>,
    pub streams: FnvHashMap<StreamNr, Arc<ReplicatedStreamInner>>,
}

/// Data for a single replicated stream, mutable state + constant data
#[derive(Debug)]
pub struct ReplicatedStreamInner {
    pub forest: Forest,
    pub validated: Variable<Tree>,
    pub incoming: Variable<Option<Link>>,
    pub latest_seen: Variable<Option<(LamportTimestamp, Offset)>>,
}

impl ReplicatedStreamInner {
    pub fn new(forest: Forest) -> Self {
        Self {
            forest,
            validated: Variable::default(),
            incoming: Variable::default(),
            latest_seen: Variable::default(),
        }
    }

    pub fn root_map_entry(&self) -> Option<RootMapEntry> {
        self.validated.project(|tree| {
            let lamport = tree.last_lamport();
            tree.link().map(|link| RootMapEntry::new(&link.into(), lamport))
        })
    }

    /// set the latest validated root
    pub fn set_latest(&self, value: Tree) {
        self.validated.set(value);
    }

    /// latest validated root
    pub fn validated(&self) -> Tree {
        self.validated.get_cloned()
    }

    /// set the latest incoming root. This will trigger validation
    pub fn set_incoming(&self, value: Link) {
        self.incoming.set(Some(value));
    }

    pub fn tree_stream(&self) -> variable::Observer<Tree> {
        self.validated.new_observer()
    }

    pub fn incoming_root_stream(&self) -> impl Stream<Item = Link> {
        self.incoming.new_observer().filter_map(future::ready)
    }
}
