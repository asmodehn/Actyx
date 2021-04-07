use axossettings::Repository;
use node::os_settings::*;
use std::collections::BTreeMap;
#[test]
fn node_schema_in_sync() {
    use maplit::btreeset;
    let sample_settings = Settings {
        swarm: Swarm {
          bootstrap_nodes: btreeset!["some bootstrap node".into()],
          announce_addresses: btreeset![],
          swarm_key: "L2tleS9zd2FybS9wc2svMS4wLjAvCi9iYXNlMTYvCjY1YjM1NDhjYTg0YWZmMTkwZjlkYTkzZThkMjQ2YWU1NjU5ZDJlZGQ1M2ZjNjQ4MjdiOWM0NTdmNWY4MzAyNGIK".into(),
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
    };
    let current_schema: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../protocols/json-schema/node-settings.schema.json"
    )))
    .unwrap();

    let mut repo = Repository::new_in_memory();
    let scope: axossettings::Scope = "com.actyx".parse().unwrap();
    repo.set_schema(&scope, current_schema).unwrap();
    repo.update_settings(&scope, serde_json::to_value(&sample_settings).unwrap(), false)
        .unwrap();
}
