use actyxos_sdk::AppId;
use crypto::PublicKey;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use util::formats::LogSeverity;

// These type definitions need to be kept in sync with the ActyxOS
// node schema, as found in [0].
// There is a somewhat simple test case in here to make sure, that
// it's mostly in sync, but subtle bugs may be introduced by
// changing the schema w/o changing the types here.

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Licensing {
    pub node: String,
    pub apps: BTreeMap<AppId, String>,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Events {
    pub read_only: bool,
    #[serde(rename = "_internal")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal: Option<serde_json::Value>,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Swarm {
    pub swarm_key: String,
    // TODO: use multiaddr
    pub bootstrap_nodes: BTreeSet<String>,
    pub announce_addresses: BTreeSet<String>,
    pub topic: String,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Admin {
    pub display_name: String,
    pub authorized_users: Vec<PublicKey>,
    pub log_levels: LogLevels,
}
#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Api {
    pub events: Events,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct LogLevels {
    pub node: LogSeverity,
}

#[derive(Serialize, Deserialize, PartialEq, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub swarm: Swarm,
    pub admin: Admin,
    pub licensing: Licensing,
    pub api: Api,
}

impl Settings {
    #[cfg(test)]
    pub fn sample() -> Self {
        use maplit::btreeset;
        Self {
            swarm: Swarm {
                swarm_key: "abcd".to_string(),
                bootstrap_nodes: btreeset!["some bootstrap node".into()],
                announce_addresses: btreeset![],
                topic: "some topic".into(),
            },
            admin: Admin {
                display_name: "some name".into(),
                log_levels: LogLevels::default(),
                authorized_users: vec![],
            },
            licensing: Licensing {
                node: "development".into(),
                apps: BTreeMap::default(),
            },
            api: Api {
                events: Events {
                    internal: None,
                    read_only: true,
                },
            },
        }
    }
}