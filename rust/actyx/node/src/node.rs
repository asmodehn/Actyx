use std::time::{Duration, Instant};

use crate::{
    components::{logging::LoggingRequest, store::StoreRequest, ComponentRequest, ComponentState, ComponentType},
    formats::{ExternalEvent, NodeDetails, NodeEvent, NodeState, ResultInspect, Settings, ShutdownReason},
    host::Host,
    node_api::formats::NodesRequest,
    settings::{SettingsRequest, SYSTEM_SCOPE},
    spawn_with_name,
};
use axossettings as settings;
use chrono::SecondsFormat;
use crossbeam::{
    channel::{bounded, Receiver, Sender},
    select,
};
use std::sync::Arc;
use thiserror::Error;
use tracing::*;
use util::formats::{node_error_context, ActyxOSCode, ActyxOSResult, ActyxOSResultExt};

pub type ApiResult<T> = ActyxOSResult<T>;

pub type NodeProcessResult<T> = std::result::Result<T, NodeError>;

#[derive(Error, Debug, Clone)]
pub enum NodeError {
    #[error("NODE_STOPPED_BY_NODE\nActyx shut down because Actyx services could not be started. Please refer to FIXME for more information. ({component}: {err:#})")]
    ServicesStartup { component: String, err: Arc<anyhow::Error> },
    #[error("NODE_STOPPED_BY_NODE\nError: internal error. Please contact Actyx support. ({0})")]
    InternalError(Arc<anyhow::Error>),
    #[error("ERR_PORT_COLLISION\nActyx shut down because it could not bind to port {port}. Please specify a different {component} port. Please refer to FIXME for more information.")]
    PortCollision { component: String, port: u16 },
}
impl From<Arc<anyhow::Error>> for NodeError {
    fn from(err: Arc<anyhow::Error>) -> Self {
        use node_error_context::*;
        match (err.downcast_ref::<Component>(), err.downcast_ref::<BindingFailed>()) {
            (Some(Component(component)), Some(BindingFailed(port))) => NodeError::PortCollision {
                component: component.into(),
                port: *port,
            },
            (Some(Component(component)), None) => NodeError::ServicesStartup {
                component: component.into(),
                err,
            },
            _ => NodeError::InternalError(err),
        }
    }
}
impl From<anyhow::Error> for NodeError {
    fn from(err: anyhow::Error) -> Self {
        Arc::new(err).into()
    }
}

trait NodeErrorResultExt<T> {
    fn internal(self) -> NodeProcessResult<T>;
}
impl<T, E: Into<anyhow::Error>> NodeErrorResultExt<T> for Result<T, E> {
    fn internal(self) -> NodeProcessResult<T> {
        self.map_err(|e| NodeError::InternalError(Arc::new(e.into())))
    }
}

struct Node {
    rx: Receiver<ExternalEvent>,
    state: NodeState,
    runtime_storage: Host,
    settings_repo: settings::Repository,
    components: Vec<ComponentChannel>,
}

impl Node {
    fn new(
        rx: Receiver<ExternalEvent>,
        components: Vec<ComponentChannel>,
        runtime_storage: Host,
    ) -> anyhow::Result<Self> {
        #[cfg(test)]
        let settings_db = axossettings::Database::in_memory()?;
        #[cfg(not(test))]
        let settings_db = axossettings::Database::new(runtime_storage.working_dir().to_owned())?;
        let mut settings_repo = settings::Repository::new(settings_db)?;
        // Apply the current schema for com.actyx.os (it might have changed). If this is
        // unsuccessful, we panic.
        apply_system_schema(&mut settings_repo).expect("Error applying system schema com.actyx.os.");

        let sys_settings: Settings = settings_repo
            .get_settings(&system_scope(), false)
            .ax_internal()
            .and_then(|s| serde_json::from_value(s).ax_internal())
            .inspect_err(|err| debug!("Unable to get initial system settings: {}", err))?;

        let node_id = runtime_storage.get_crypto_cell().get_or_create_node_id()?;
        let state = NodeState::new(node_id, sys_settings);
        Ok(Self {
            settings_repo,
            runtime_storage,
            state,
            components,
            rx,
        })
    }
}

fn system_scope() -> settings::Scope {
    SYSTEM_SCOPE.parse().unwrap()
}

fn is_system_scope(scope: &settings::Scope) -> bool {
    scope.first() == Some(SYSTEM_SCOPE.to_string())
}

/// Set the schema for the ActyxOS system settings.
pub fn apply_system_schema(settings_repo: &mut settings::Repository) -> settings::repository::Result<()> {
    debug!("setting current schema for com.actyx.os");
    let schema: serde_json::Value = serde_json::from_slice(include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../protocols/json-schema/os/node-settings.schema.json"
    )))
    .expect("embedded settings schema is not valid json");
    // check that embedded schema for com.actyx.os is a valid schema. If not, there is no point in going on.
    settings::Validator::new(schema.clone()).expect("Embedded schema for com.actyx.os is not a valid JSON schema.");

    settings_repo.set_schema(&system_scope(), schema)?;
    Ok(())
}

macro_rules! standard_lifecycle {
    ($m:expr, $s:expr) => {
        match &$m {
            NodeEvent::Shutdown(r) => $s.send(ComponentRequest::Shutdown(r.clone()))?,
            NodeEvent::StateUpdate(NodeState { settings, .. }) => {
                $s.send(ComponentRequest::SettingsChanged(Box::new(settings.clone())))?
            }
        }
    };
}

impl Node {
    fn handle_set_settings_request(
        &mut self,
        scope: &settings::Scope,
        json: serde_json::Value,
        ignore_errors: bool,
    ) -> ApiResult<serde_json::Value> {
        if scope.is_root() {
            return Err(ActyxOSCode::ERR_INVALID_INPUT
                .with_message("You cannot set settings for the root scope. Please specify a settings scope."));
        }
        debug!("Trying to set settings for {}", scope);
        let update = if is_system_scope(&scope) && ignore_errors {
            debug!("Ignoring force option for system scope.");
            self.settings_repo.update_settings(&scope, json, false)?
        } else {
            self.settings_repo.update_settings(&scope, json, ignore_errors)?
        };
        if is_system_scope(&scope) {
            self.update_node_state()?;
        }
        Ok(update)
    }

    fn handle_unset_settings_request(&mut self, scope: &settings::Scope) -> ApiResult<()> {
        debug!("Trying to unset settings for {}", scope);
        self.settings_repo.clear_settings(scope).ax_internal()?;
        self.update_node_state()?;
        Ok(())
    }

    fn handle_settings_request(&mut self, request: SettingsRequest) {
        match request {
            SettingsRequest::SetSettings {
                scope,
                json,
                response,
                ignore_errors,
            } => {
                let res = self
                    .handle_set_settings_request(&scope, json, ignore_errors)
                    .inspect_err(|e| debug!("Error handling set settings request: {}", e));
                if res.is_ok() {
                    info!(target: "NODE_SETTINGS_CHANGED", "Node settings at scope {} were changed. Please refer to FIXME for more information.", scope);
                }
                let _ = response.send(res);
            }
            SettingsRequest::UnsetSettings { response, scope } => {
                let res = self
                    .handle_unset_settings_request(&scope)
                    .inspect_err(|e| debug!("Error handling unset settings request: {}", e));
                let _ = response.send(res);
            }
            SettingsRequest::GetSettings {
                scope,
                response,
                no_defaults,
            } => {
                let res = self.settings_repo.get_settings(&scope, no_defaults).map_err(Into::into);
                let _ = response.send(res);
            }
            SettingsRequest::SetSchema { scope, json, response } => {
                let res = self.settings_repo.set_schema(&scope, json).map_err(Into::into);
                let _ = response.send(res);
            }
            SettingsRequest::DeleteSchema { scope, response } => {
                let res = self.settings_repo.delete_schema(&scope).map_err(Into::into);
                let _ = response.send(res);
            }
            SettingsRequest::GetSchemaScopes { response } => {
                let res = self.settings_repo.get_schema_scopes().map_err(Into::into);
                let _ = response.send(res);
            }
            SettingsRequest::GetSchema { scope, response } => {
                let res = self.settings_repo.get_schema(&scope).map_err(Into::into);
                let _ = response.send(res);
            }
        };
    }
    fn handle_nodes_request(&self, request: NodesRequest) {
        match request {
            NodesRequest::Ls(sender) => {
                let resp = util::formats::NodesLsResponse {
                    node_id: self.state.details.node_id,
                    display_name: self.state.details.node_name.to_string(),
                    version: env!("CARGO_PKG_VERSION").into(),
                    started_unix: self.state.started_at.timestamp(),
                    started_iso: self.state.started_at.to_rfc3339_opts(SecondsFormat::Secs, false),
                };
                debug!("NodesLsResponse: {:?}", resp);
                let _ = sender.send(Ok(resp));
            }
            NodesRequest::GetNodeId(sender) => {
                let _ = sender.send(
                    self.runtime_storage
                        .get_crypto_cell()
                        .get_or_create_node_id()
                        .map(Into::into),
                );
            }
        }
    }
    fn update_node_state(&mut self) -> ActyxOSResult<()> {
        let node_settings = self.settings_repo.get_settings(&system_scope(), false)?;
        eprintln!("node_settings {}", node_settings);
        let settings = serde_json::from_value(node_settings).ax_internal()?;
        if settings != self.state.settings {
            let details = NodeDetails::from_settings(
                &settings,
                self.runtime_storage.get_crypto_cell().get_or_create_node_id()?,
            );
            debug!("Setting node settings to: {:?}", settings);
            self.state.settings = settings;
            self.state.details = details;
            self.send(NodeEvent::StateUpdate(self.state.clone()))?;
        }
        Ok(())
    }

    fn send(&mut self, message: NodeEvent) -> ActyxOSResult<()> {
        debug!("Node event {:?}", message);
        for c in &self.components {
            match c {
                ComponentChannel::Store(s) => standard_lifecycle!(message, s),
                ComponentChannel::NodeApi(s) => standard_lifecycle!(message, s),
                ComponentChannel::Logging(s) => standard_lifecycle!(message, s),
                ComponentChannel::Android(s) => standard_lifecycle!(message, s),
                #[cfg(test)]
                ComponentChannel::Test(s) => standard_lifecycle!(message, s),
            }
        }
        Ok(())
    }

    fn register_with_components(&mut self, tx: Sender<(ComponentType, ComponentState)>) -> anyhow::Result<()> {
        for c in &self.components {
            match c {
                ComponentChannel::Store(s) => s.send(ComponentRequest::RegisterSupervisor(tx.clone()))?,
                ComponentChannel::NodeApi(s) => s.send(ComponentRequest::RegisterSupervisor(tx.clone()))?,
                ComponentChannel::Logging(s) => s.send(ComponentRequest::RegisterSupervisor(tx.clone()))?,
                ComponentChannel::Android(s) => s.send(ComponentRequest::RegisterSupervisor(tx.clone()))?,
                #[cfg(test)]
                ComponentChannel::Test(s) => s.send(ComponentRequest::RegisterSupervisor(tx.clone()))?,
            }
        }

        Ok(())
    }

    fn run(mut self) -> NodeProcessResult<()> {
        let (tx, component_rx) = bounded(256);
        self.register_with_components(tx).internal()?;

        self.send(NodeEvent::StateUpdate(self.state.clone())).internal()?;
        tracing::info!(target: "NODE_STARTED_BY_HOST", "Actyx is running.");

        let exit_result = loop {
            select! {
                recv(self.rx) -> msg => {
                    let event = msg.internal()?;
                    let result = match event {
                        ExternalEvent::NodesRequest(req) => {
                            self.handle_nodes_request(req);
                            Ok(())
                        },
                        ExternalEvent::SettingsRequest(req) => {
                            self.handle_settings_request(req);
                            Ok(())
                        },
                        ExternalEvent::ShutdownRequested(r) => {
                            // Signal shutdown right away, otherwise the logging component might
                            // have already stopped
                            let exit_result = match r {
                                ShutdownReason::TriggeredByHost => {
                                    info!(target: "NODE_STOPPED_BY_HOST", "Actyx is stopped. The shutdown was either initiated automatically by the host or intentionally by the user. Please refer to FIXME for more information.");
                                    Ok(())
                                }
                                ShutdownReason::TriggeredByUser => {
                                    info!(target: "NODE_STOPPED_BY_NODEUI", "Actyx is stopped. The shutdown initiated by the user. Please refer to FIXME for more information.");
                                    Ok(())
                                }
                                ShutdownReason::Internal(ref err) => {
                                    error!(target: "NODE_STOPPED_BY_NODE", "{}", err);
                                    Err(err.clone())
                                }
                            };
                            // Notify all registered components
                            self.send(NodeEvent::Shutdown(r)).internal()?;
                            break exit_result;
                        }
                    };
                    if let Err(e) = result {
                        error!("Unrecoverable error, exiting ({})", e);
                        return Err(e);
                    }
                },
                recv(component_rx) -> msg => {
                    let (from_component, new_state) = msg.internal()?;
                    debug!("Received component state transition: {} {:?}", from_component, new_state);
                    if let ComponentState::Errored(e) = new_state {
                        warn!("Shutting down because component {} errored: \"{}\"", from_component, e);
                        let err: NodeError = e.context(node_error_context::Component(format!("{}", from_component))).into();

                        error!(target: "NODE_STOPPED_BY_NODE", "{}", err);
                        self.send(NodeEvent::Shutdown(ShutdownReason::Internal(err.clone()))).internal()?;
                        return Err(err);
                    }
                }
            }
        };

        // Wait for registered components to stop, at most 500 ms
        let mut stopped_components = 0;
        let start = Instant::now();
        while stopped_components < self.components.len()
            && (Instant::now().duration_since(start) < Duration::from_millis(500))
        {
            if let Ok((_, ComponentState::Stopped)) = component_rx.recv_timeout(Duration::from_millis(500)) {
                stopped_components += 1;
            }
        }

        exit_result
    }
}

pub(crate) enum ComponentChannel {
    Store(Sender<ComponentRequest<StoreRequest>>),
    NodeApi(Sender<ComponentRequest<()>>),
    Logging(Sender<ComponentRequest<LoggingRequest>>),
    Android(Sender<ComponentRequest<()>>),
    #[cfg(test)]
    Test(Sender<ComponentRequest<()>>),
}

pub struct NodeWrapper {
    /// Cloneable sender to interact with the `Node`
    pub tx: Sender<ExternalEvent>,
    /// One shot receiver; message indicating an exit of `Node`
    pub rx_process: Option<Receiver<NodeProcessResult<()>>>,
}

impl NodeWrapper {
    pub(crate) fn new(
        (tx, rx): (Sender<ExternalEvent>, Receiver<ExternalEvent>),
        components: Vec<ComponentChannel>,
        runtime_storage: Host,
    ) -> anyhow::Result<Self> {
        let node = Node::new(rx, components, runtime_storage)?;
        let (tx_process, rx_process) = bounded(1);
        let _ = spawn_with_name("NodeLifecycle", move || {
            let r = node.run();
            if let Err(e) = &r {
                eprintln!("Node exited with error {}", e);
            }
            tx_process.send(r).expect("Error sending result from NodeLifecycle");
        });
        Ok(Self {
            tx,
            rx_process: Some(rx_process),
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::components::Component;
    use anyhow::Result;
    use axossettings as settings;
    use futures::executor::block_on;
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::sync::oneshot::channel;
    use util::formats::NodeName;

    #[tokio::test]
    async fn should_handle_settings_requests() -> ActyxOSResult<()> {
        let (_runtime_tx, runtime_rx) = crossbeam::channel::bounded(8);
        let temp_dir = TempDir::new().unwrap();
        let runtime = Host::new(temp_dir.path().to_path_buf());
        let mut node = Node::new(runtime_rx, vec![], runtime).unwrap();
        let schema = serde_json::from_slice(include_bytes!(
            "../../../../protocols/json-schema/os/node-settings.schema.json"
        ))
        .unwrap();
        let scope = system_scope();
        let json = json!({ "general": { "displayName": "My Node", "swarmKey": "L2tleS9zz2FybS9zz2zzzS4wLjAvCi9iYXNlMTYvCjRkNjMzzzzzNjQ0YzY3NDgzMjRjNzczNTM4NzIzMDU3NDQ0NDZjNTU2ODYyMzEzMzRjNzEzNjRmNzc2OTMzNjg=", "bootstrapNodes": [ "/ip4/127.0.0.1/tcp/4001/ipfs/QmaAxuktPMR3ESHe9Pru8kzzzSGvsUie7UFJPfCWqTzzzz" ], "announceAddresses": [], "authorizedKeys": [], "logLevels": { "os": "WARN", "apps": "INFO" } }, "licensing": { "os": "development", "apps": { "com.example.sample": "development" } }, "services": { "eventService": { "topic": "My Topic", "readOnly": false, "_internal": { "allow_publish": true, "topic": "actyxos-demo" } } } });

        // Set the schema for `com.actyx.os
        {
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::SetSchema {
                scope: scope.clone(),
                json: schema,
                response,
            });

            rx.await.ax_internal()??;
        }
        // Set settings for `com.actyx.os`
        {
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::SetSettings {
                scope,
                json: json.clone(),
                response,
                ignore_errors: false,
            });

            assert_eq!(json, rx.await.ax_internal()??);
            assert_eq!(node.state.settings, serde_json::from_value(json).unwrap());
            assert_eq!(node.state.details.node_name, NodeName("My Node".into()));
        }
        // Set settings for `com.actyx.os/general/displayName`
        {
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::GetSettings {
                scope: "com.actyx.os/general/displayName".parse().unwrap(),
                no_defaults: false,
                response,
            });
            assert_eq!("My Node", rx.await.ax_internal()??);
        }
        {
            let changed = serde_json::json!("changed");
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::SetSettings {
                scope: "com.actyx.os/general/displayName".parse().unwrap(),
                json: changed.clone(),
                response,
                ignore_errors: false,
            });

            assert_eq!(
                *rx.await
                    .ax_internal()??
                    .pointer("/general/displayName")
                    .to_owned()
                    .unwrap(),
                changed
            );
        }
        {
            let invalid = serde_json::json!("not_valid");
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::SetSettings {
                scope: "com.actyx.os/licensing/os".parse().unwrap(),
                json: invalid,
                response,
                ignore_errors: false, // <=========
            });
            assert_eq!(
                rx.await.ax_internal()?,
                Err(ActyxOSCode::ERR_SETTINGS_INVALID
                    .with_message("Validation failed.\n\tErrors:\n\t\t/licensing/os: OneOf conditions are not met."))
            );
        }
        {
            // Setting invalid values for `com.actyx.os` is not allowed
            let invalid = serde_json::json!("not_valid");
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::SetSettings {
                scope: "com.actyx.os/licensing/os".parse().unwrap(),
                json: invalid,
                response,
                ignore_errors: true, // <=========
            });
            assert_eq!(
                rx.await.ax_internal()?,
                Err(ActyxOSCode::ERR_SETTINGS_INVALID
                    .with_message("Validation failed.\n\tErrors:\n\t\t/licensing/os: OneOf conditions are not met."))
            )
        }
        {
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::UnsetSettings {
                scope: settings::Scope::root(),
                response,
            });
            assert!(rx.await.ax_internal()?.is_ok());
        }
        {
            let json = serde_json::json!(null);
            let (response, rx) = channel();
            node.handle_settings_request(SettingsRequest::SetSettings {
                scope: settings::Scope::root(),
                json,
                response,
                ignore_errors: false,
            });
            assert_eq!(
                rx.await.ax_internal()?,
                Err(ActyxOSCode::ERR_INVALID_INPUT
                    .with_message("You cannot set settings for the root scope. Please specify a settings scope."))
            );
        }
        Ok(())
    }

    struct DummyComponent {
        node_rx: Receiver<ComponentRequest<()>>,
    }
    impl Component<(), ()> for DummyComponent {
        fn get_type(&self) -> &'static str {
            "test"
        }
        fn get_rx(&self) -> &Receiver<ComponentRequest<()>> {
            &self.node_rx
        }
        fn handle_request(&mut self, _: ()) -> Result<()> {
            Ok(())
        }
        fn extract_settings(&self, _: Settings) -> Result<()> {
            Ok(())
        }
        fn set_up(&mut self, _: ()) -> bool {
            true
        }
        fn start(&mut self, _: Sender<anyhow::Error>) -> Result<()> {
            Ok(())
        }
        fn stop(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn handle_component_lifecycle() -> anyhow::Result<()> {
        // Bootstrap
        let (node_tx, node_rx) = crossbeam::channel::bounded(512);
        let (component_tx, component_rx) = crossbeam::channel::bounded(512);
        let host = Host::new(std::env::current_dir()?);
        let node = NodeWrapper::new(
            (node_tx.clone(), node_rx),
            vec![ComponentChannel::Test(component_tx)],
            host,
        )?;

        // should provide rx_process and not exit immediately
        assert!(node.rx_process.as_ref().unwrap().try_recv().is_err());

        // should register with Component
        let _component_state_tx = match component_rx.recv()? {
            ComponentRequest::RegisterSupervisor(snd) => snd,
            _ => panic!(),
        };

        // should emit initial state
        assert!(matches!(component_rx.recv()?, ComponentRequest::SettingsChanged(_)));

        // shutdown
        node_tx.send(ExternalEvent::ShutdownRequested(ShutdownReason::TriggeredByHost))?;
        // forward shutdown request to component
        assert!(matches!(component_rx.recv()?, ComponentRequest::Shutdown(_)));
        // yield on `rx_process`
        node.rx_process.unwrap().recv()??;
        Ok(())
    }

    #[test]
    fn exit_on_component_error() -> anyhow::Result<()> {
        // Bootstrap
        let (node_tx, node_rx) = crossbeam::channel::bounded(512);
        let (component_tx, component_rx) = crossbeam::channel::bounded(512);
        let host = Host::new(std::env::current_dir()?);
        let node = NodeWrapper::new((node_tx, node_rx), vec![ComponentChannel::Test(component_tx)], host)?;

        // should register with Component
        let component_state_tx = match component_rx.recv()? {
            ComponentRequest::RegisterSupervisor(snd) => snd,
            _ => panic!(),
        };

        // should emit initial state
        assert!(matches!(component_rx.recv()?, ComponentRequest::SettingsChanged(_)));

        component_state_tx.send((
            "test".to_string().into(),
            ComponentState::Errored(anyhow::anyhow!("Unrecoverable")),
        ))?;

        // send shutdown request to component
        assert!(matches!(component_rx.recv()?, ComponentRequest::Shutdown(_)));
        // yield on `rx_process` and forward component's error
        matches!(node.rx_process.unwrap().recv()?, Err(NodeError::ServicesStartup { .. }));
        Ok(())
    }

    #[test]
    fn change_and_forward_settings() -> anyhow::Result<()> {
        // Bootstrap
        let (node_tx, node_rx) = crossbeam::channel::bounded(512);
        let (component_tx, component_rx) = crossbeam::channel::bounded(512);
        let host = Host::new(std::env::current_dir()?);
        let node = NodeWrapper::new((node_tx, node_rx), vec![ComponentChannel::Test(component_tx)], host)?;

        // should register with Component
        let _component_state_tx = match component_rx.recv()? {
            ComponentRequest::RegisterSupervisor(snd) => snd,
            _ => panic!(),
        };

        // should emit initial state
        let mut settings = match component_rx.recv()? {
            ComponentRequest::SettingsChanged(s) => s,
            _ => panic!(),
        };

        settings.general.display_name = "Changed".into();

        let (req_tx, req_rx) = tokio::sync::oneshot::channel();
        let json = serde_json::to_value(&*settings)?;
        node.tx
            .send(ExternalEvent::SettingsRequest(SettingsRequest::SetSettings {
                ignore_errors: false,
                json: json.clone(),
                scope: system_scope(),
                response: req_tx,
            }))?;
        assert_eq!(block_on(req_rx)??, json);

        let set_up = match component_rx.recv()? {
            ComponentRequest::SettingsChanged(s) => s,
            _ => panic!(),
        };
        assert_eq!(settings, set_up);

        // shutdown
        node.tx
            .send(ExternalEvent::ShutdownRequested(ShutdownReason::TriggeredByHost))?;
        // forward shutdown request to component
        assert!(matches!(component_rx.recv()?, ComponentRequest::Shutdown(_)));
        // yield on `rx_process`
        node.rx_process.unwrap().recv()??;
        Ok(())
    }
}
