use crate::event::scalars::{SourceId, MAX_SOURCEID_LENGTH};
use anyhow::{anyhow, bail, ensure, Context, Result};
use multibase::Base;
use serde::{Deserialize, Serialize};
use std::{
    convert::TryFrom,
    fmt::{Debug, Display},
};

/// The session identifier used in subscribeUntilTimeTravel
#[derive(Debug, Clone, Serialize, Deserialize, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SessionId(Box<str>);

impl Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&*self.0)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.into())
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl SessionId {
    /// Extracts a string slice containing the entire session id
    pub fn as_str(&self) -> &str {
        &*self.0
    }
}

/// Macro for constructing an [`AppId`](tagging/struct.AppId.html) literal.
///
/// This is how it works:
/// ```no_run
/// use actyxos_sdk::{app_id, tagged::AppId};
/// let app_id: AppId = app_id!("abc");
/// ```
/// This does not compile:
/// ```compile_fail
/// use actyxos_sdk::{app_id, tagged::AppId};
/// let app_id: AppId = app_id!("");
/// ```
#[macro_export]
macro_rules! app_id {
    ($lit:tt) => {{
        #[allow(dead_code)]
        type X = $crate::assert_len!($lit, 1..);
        use ::std::convert::TryFrom;
        $crate::tagged::AppId::try_from($lit).unwrap()
    }};
}

mk_scalar!(
    /// The app ID denotes a specific app (sans versioning)
    ///
    /// This is used for marking the provenance of events as well as configuring access rights.
    struct AppId, EmptyAppId
);

/// The ActyxOS node identifier
///
/// Each ActyxOS node has a private key that defines its identity. The corresponding public
/// key uniquely identifies the node but depends on the used crypto scheme. Therefore, we
/// use a 256bit hash of the public key as NodeId.
///
/// Values of this type serialize as Base64url multibase strings by default.
/// Deserialization is supported from binary data or multibase format.
///
/// Each node may emit multiple sources, each identified by its own [`StreamId`](struct.StreamId.html).
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(into = "String", try_from = "String")]
pub struct NodeId([u8; 32]);

impl NodeId {
    pub fn to_multibase(&self, base: Base) -> String {
        multibase::encode(base, self.0)
    }

    pub fn from_multibase(input: impl AsRef<str>) -> Result<NodeId> {
        let (_base, bytes) = multibase::decode(input).context("deserializing NodeId")?;
        if bytes.len() == 32 {
            let mut id = [0u8; 32];
            id.copy_from_slice(&bytes[..32]);
            Ok(Self(id))
        } else {
            Err(anyhow!("invalid NodeId length: {}", bytes.len()))
        }
    }

    /// Creates a stream ID belonging to this node ID with the given non-zero stream number
    ///
    /// Stream number zero is reserved for embedding [`SourceId`](../event/struct.SourceId.html).
    pub fn stream(&self, stream_nr: u64) -> Option<StreamId> {
        if stream_nr == 0 {
            None
        } else {
            Some(StreamId {
                node_id: *self,
                stream_nr,
            })
        }
    }
}

impl AsRef<[u8]> for NodeId {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_multibase(Base::Base64Url))
    }
}

impl Into<String> for NodeId {
    fn into(self) -> String {
        self.to_string()
    }
}

impl TryFrom<&str> for NodeId {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_multibase(value)
    }
}

impl TryFrom<String> for NodeId {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_multibase(value)
    }
}

/// The unique identifier of a single event stream emitted by an ActyxOS node
///
/// The emitting node — identified by its [`NodeId`](struct.NodeId.html) — may emit multiple
/// streams with different IDs. The emitting node’s ID can be extracted from this stream ID
/// without further information.
///
/// The default serialization of this type is the string representation of the NodeId
/// followed by a dot and a base64url multibase-encoded multiformats-varint (see also
/// [`varint`](../varint)), unless the stream number is zero, in which case the node ID
/// is interpreted as a [`SourceId`](../event/struct.SourceId.html).
#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "dataflow", derive(Abomonation))]
#[serde(into = "String", try_from = "String")]
pub struct StreamId {
    node_id: NodeId,
    stream_nr: u64,
}

impl StreamId {
    pub fn node_id(&self) -> Option<NodeId> {
        if self.stream_nr == 0 {
            None
        } else {
            Some(self.node_id)
        }
    }

    pub fn stream_nr(&self) -> u64 {
        self.stream_nr
    }

    pub fn to_source_id(self) -> Result<SourceId, anyhow::Error> {
        if self.stream_nr == 0 {
            let mut bytes = [0u8; MAX_SOURCEID_LENGTH + 1];
            bytes.copy_from_slice(&self.node_id.as_ref()[0..=MAX_SOURCEID_LENGTH]);
            Ok(SourceId(bytes))
        } else {
            Err(anyhow!(
                "can only convert StreamId with stream number zero to SourceId"
            ))
        }
    }

    fn parse_str(value: &str) -> Result<Self> {
        let mut split = value.split('.');
        let node_str = split
            .next()
            .ok_or_else(|| anyhow!("no NodeId in serialized StreamId"))?;
        let stream_str = split
            .next()
            .ok_or_else(|| anyhow!("no stream nr in serialized StreamId"))?;
        if split.next().is_some() {
            bail!("trailing garbage in StreamId");
        }
        let node_id = NodeId::try_from(node_str)?;
        let stream_nr = stream_str
            .parse()
            .context("parsing StreamId stream number")?;
        ensure!(stream_nr != 0, "invalid stream nr in StreamId");
        Ok(Self { node_id, stream_nr })
    }
}

impl Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Ok(source_id) = self.to_source_id() {
            f.write_str(source_id.as_str())
        } else {
            write!(f, "{}.{}", self.node_id, self.stream_nr)
        }
    }
}

impl Into<String> for StreamId {
    fn into(self) -> String {
        self.to_string()
    }
}

impl TryFrom<&str> for StreamId {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse_str(value).or_else(|_| {
            Ok(SourceId::try_from(value)
                .context("parsing StreamId")?
                .into())
        })
    }
}

impl TryFrom<String> for StreamId {
    type Error = anyhow::Error;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<SourceId> for StreamId {
    fn from(src: SourceId) -> Self {
        Self::from(&src)
    }
}

impl From<&SourceId> for StreamId {
    fn from(src: &SourceId) -> Self {
        let mut bytes = [0u8; 32];
        bytes[0..=MAX_SOURCEID_LENGTH].copy_from_slice(&src.0[..]);
        StreamId {
            node_id: NodeId(bytes),
            stream_nr: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck::Arbitrary;
    use serde_json::Value;

    impl Arbitrary for NodeId {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let mut bytes = [0u8; 32];
            g.fill_bytes(&mut bytes[..]);
            NodeId(bytes)
        }
    }

    impl Arbitrary for StreamId {
        fn arbitrary<G: quickcheck::Gen>(g: &mut G) -> Self {
            let stream_nr = u64::arbitrary(g);
            if stream_nr == 0 {
                SourceId::arbitrary(g).into()
            } else {
                Self {
                    node_id: NodeId::arbitrary(g),
                    stream_nr,
                }
            }
        }
    }

    const BYTES: [u8; 32] = [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
    ];

    #[test]
    fn node_id_serialization() {
        let node_id = NodeId(BYTES);
        assert_eq!(
            node_id.to_string(),
            "uAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA"
        );
    }

    #[test]
    fn stream_id_serialization() {
        let stream_id = NodeId(BYTES).stream(12).unwrap();
        assert_eq!(
            stream_id.to_string(),
            "uAQIDBAUGBwgJCgsMDQ4PEBESExQVFhcYGRobHB0eHyA.12"
        );
    }

    #[test]
    fn quick1() {
        let sid = StreamId {
            node_id: NodeId([
                81, 66, 94, 87, 52, 39, 60, 110, 43, 93, 98, 94, 97, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0,
            ]),
            stream_nr: 0,
        };
        assert_eq!(
            serde_json::to_value(&sid).unwrap(),
            Value::String(sid.to_string())
        );
    }

    quickcheck::quickcheck! {
        fn node_id_roundtrip(id: NodeId) -> bool {
            let s = id.to_string();
            NodeId::try_from(s).map_err(|_| "") == Ok(id)
        }

        fn stream_id_roundtrip(sid: StreamId) -> bool {
            let s = sid.to_string();
            StreamId::try_from(s).map_err(|_| "") == Ok(sid)
        }

        fn stream_id_to_string(sid: StreamId) -> bool {
            serde_json::to_value(&sid).unwrap() == Value::String(sid.to_string())
        }

        fn source_id_serialization(src: SourceId) -> bool {
            let stream = StreamId::from(src);
            serde_json::to_string(&src).map_err(|_| "a") == serde_json::to_string(&stream).map_err(|_| "b")
        }

        fn source_id_deserialization(src: SourceId) -> bool {
            let s = src.to_string();
            SourceId::try_from(&*s).map_err(|_| "a").map(StreamId::from)
              == StreamId::try_from(s).map_err(|_| "b")
        }

        fn source_id_roundtrip(src: SourceId) -> bool {
            Ok(src) == StreamId::from(src).to_source_id().map_err(|_| "")
        }
    }
}
