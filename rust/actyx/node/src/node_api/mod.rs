use crate::{
    components::{
        node_api::NodeApiSettings,
        store::{StoreRequest, StoreTx},
        ComponentRequest,
    },
    formats::ExternalEvent,
    settings::{SettingsRequest, SYSTEM_SCOPE},
};
use anyhow::Context;
use crossbeam::channel::Sender;
use crypto::PublicKey;
use formats::NodesRequest;
use futures::{
    future::BoxFuture,
    pin_mut,
    stream::FuturesUnordered,
    task::{self, Poll},
    Future, FutureExt, StreamExt,
};
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::Boxed},
    identity,
    multiaddr::Protocol,
    ping::{Ping, PingConfig, PingEvent},
    swarm::{
        IntoProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction, NetworkBehaviourEventProcess, PollParameters,
        ProtocolsHandler, Swarm, SwarmBuilder, SwarmEvent,
    },
    Multiaddr, NetworkBehaviour, PeerId,
};
use libp2p_streaming_response::{ChannelId, StreamingResponse, StreamingResponseConfig};
use parking_lot::Mutex;
use std::{collections::BTreeSet, convert::TryFrom, pin::Pin, sync::Arc, time::Duration};
use tracing::*;
use util::formats::{
    admin_protocol::{AdminProtocol, AdminRequest, AdminResponse},
    ActyxOSCode, ActyxOSError, ActyxOSResult, ActyxOSResultExt, NodeErrorContext, NodesInspectResponse,
};
use util::SocketAddrHelper;

pub mod formats;

type PendingRequest = BoxFuture<'static, (ChannelId, ActyxOSResult<AdminResponse>)>;

struct State {
    node_tx: Sender<ExternalEvent>,
    auth_info: Arc<Mutex<NodeApiSettings>>,
    store: StoreTx,
    /// Pending inflight requests to Node.
    pending_oneshot: FuturesUnordered<PendingRequest>,
    admin_sockets: BTreeSet<Multiaddr>,
}
#[derive(NetworkBehaviour)]
#[behaviour(poll_method = "poll", out_event = "()")]
pub struct ApiBehaviour {
    admin: StreamingResponse<AdminProtocol>,
    ping: Ping,
    #[behaviour(ignore)]
    state: State,
}
type WrappedBehaviour = Swarm<ApiBehaviour>;

macro_rules! request_oneshot {
    ($channel_id:expr, $slf:expr, $build_request:expr, $result:expr) => {{
        let maybe_add_key = $slf.maybe_add_key($channel_id.peer());
        let (tx, rx) = tokio::sync::oneshot::channel();
        $slf.state.node_tx.send($build_request(tx)).unwrap();
        let fut = async move {
            if let Err(e) = maybe_add_key.await {
                error!("Error adding initial key {}", e);
            }

            let result = rx
                .await
                .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error waiting for response")
                .and_then(|x| x.map($result));

            ($channel_id, result)
        }
        .boxed();
        $slf.state.pending_oneshot.push(fut);
    }};
}

impl ApiBehaviour {
    fn new(node_tx: Sender<ExternalEvent>, store: StoreTx, auth_info: Arc<Mutex<NodeApiSettings>>) -> Self {
        let state = State {
            node_tx,
            store,
            auth_info,
            pending_oneshot: FuturesUnordered::new(),
            admin_sockets: BTreeSet::new(),
        };
        Self {
            ping: Ping::new(PingConfig::new().with_keep_alive(true)),
            admin: StreamingResponse::new(StreamingResponseConfig::default()),
            state,
        }
    }

    /// Checks whether `peer` is authorized to use this API. If there are no
    /// authorized keys, any connected peer is authorized.
    fn is_authorized(&self, peer: &PeerId) -> bool {
        let g = self.state.auth_info.lock();
        g.authorized_keys.is_empty() || g.authorized_keys.contains(peer)
    }

    fn maybe_add_key(&self, peer: PeerId) -> BoxFuture<'static, ActyxOSResult<()>> {
        let mut auth_info = self.state.auth_info.lock();
        if auth_info.authorized_keys.is_empty() {
            match PublicKey::try_from(peer) {
                Ok(key_id) => {
                    debug!("Adding {} (peer {}) to authorized users", key_id, peer);
                    // Directly add the peer. This will be overridden as soon as the settings round
                    // tripped.
                    auth_info.authorized_keys.push(peer);
                    drop(auth_info);
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    self.state
                        .node_tx
                        .send(ExternalEvent::SettingsRequest(SettingsRequest::SetSettings {
                            scope: format!("{}/admin/authorizedUsers", SYSTEM_SCOPE).parse().unwrap(),
                            ignore_errors: false,
                            json: serde_json::json!([format!("{}", key_id)]),
                            response: tx,
                        }))
                        .unwrap();
                    async move {
                        rx.await
                            .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error waiting for response")
                            .and_then(|x| {
                                x.map(|_| {
                                    info!(
                                        "User with public key {} has been added as the first authorized user.",
                                        key_id
                                    );
                                })
                            })
                    }
                    .boxed()
                }
                Err(e) => {
                    async move { Err(ActyxOSError::internal(format!("Error converting to PublicKey: {}", e))) }.boxed()
                }
            }
        } else {
            async move { Ok(()) }.boxed()
        }
    }

    // Assumes peer is authorized
    fn enqueue(&mut self, channel_id: ChannelId, request: AdminRequest) {
        match request {
            AdminRequest::NodesLs => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::NodesRequest(NodesRequest::Ls(tx)),
                AdminResponse::NodesLsResponse
            ),

            AdminRequest::SettingsGet { no_defaults, scope } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSettings {
                    no_defaults,
                    scope,
                    response: tx
                }),
                AdminResponse::SettingsGetResponse
            ),
            AdminRequest::SettingsSchema { scope } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchema { scope, response: tx }),
                AdminResponse::SettingsSchemaResponse
            ),
            AdminRequest::SettingsScopes => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::GetSchemaScopes { response: tx }),
                AdminResponse::SettingsScopesResponse
            ),
            AdminRequest::SettingsSet {
                ignore_errors,
                json,
                scope,
            } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::SetSettings {
                    scope,
                    json,
                    ignore_errors,
                    response: tx
                }),
                AdminResponse::SettingsSetResponse
            ),
            AdminRequest::SettingsUnset { scope } => request_oneshot!(
                channel_id,
                self,
                |tx| ExternalEvent::SettingsRequest(SettingsRequest::UnsetSettings { scope, response: tx }),
                |_| AdminResponse::SettingsUnsetResponse
            ),
            AdminRequest::NodesInspect => {
                let (tx, rx) = tokio::sync::oneshot::channel();
                self.state
                    .store
                    .send(ComponentRequest::Individual(StoreRequest::NodesInspect { tx }))
                    .unwrap();
                let maybe_add_key = self.maybe_add_key(channel_id.peer());
                let admin_addrs = self.state.admin_sockets.iter().map(|a| a.to_string()).collect();
                let fut = async move {
                    if let Err(e) = maybe_add_key.await {
                        error!("Error adding initial key {}", e);
                    }
                    let res = rx
                        .await
                        .ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error waiting for response")
                        .and_then(|x| {
                            x.ax_err_ctx(ActyxOSCode::ERR_INTERNAL_ERROR, "Error getting swarm state")
                                .map(|res| {
                                    AdminResponse::NodesInspectResponse(NodesInspectResponse {
                                        peer_id: res.peer_id,
                                        swarm_addrs: res.swarm_addrs,
                                        announce_addrs: res.announce_addrs,
                                        admin_addrs,
                                        connections: res.connections,
                                        known_peers: res.known_peers,
                                    })
                                })
                        });
                    (channel_id, res)
                }
                .boxed();
                self.state.pending_oneshot.push(fut);
            }
        }
    }

    /// The main purpose of this function is to shovel responses from any
    /// pending requests to libp2p.
    fn poll(&mut self, cx: &mut task::Context, _: &mut impl PollParameters) ->
    Poll<NetworkBehaviourAction<<<<Self as
    NetworkBehaviour>::ProtocolsHandler as IntoProtocolsHandler>::Handler as
    ProtocolsHandler>::InEvent, ()>>{
        let mut wake_me_up = false;

        // Handle pending requests
        while let Poll::Ready(Some((chan, resp))) = self.state.pending_oneshot.poll_next_unpin(cx) {
            if self.admin.respond_final(chan, resp).is_err() {
                debug!("Client dropped request");
            }
            wake_me_up = true;
        }

        // This `poll` function is the last in the derived NetworkBehaviour.
        // This means, when interacting with any sub-behaviours here, we have to
        // make sure that they are being polled again. This smells, but it is a
        // limitation or design flaw within libp2p. Not much we can do about it
        // here.
        if wake_me_up {
            cx.waker().wake_by_ref();
        }

        Poll::Pending
    }
}

impl NetworkBehaviourEventProcess<libp2p_streaming_response::StreamingResponseEvent<AdminProtocol>> for ApiBehaviour {
    fn inject_event(&mut self, event: libp2p_streaming_response::StreamingResponseEvent<AdminProtocol>) {
        debug!("Received streaming_response event: {:?}", event);

        match event {
            libp2p_streaming_response::StreamingResponseEvent::<AdminProtocol>::ReceivedRequest {
                payload,
                channel_id,
            } => {
                let peer = channel_id.peer();
                if !self.is_authorized(&peer) {
                    warn!("Received unauthorized request from {}. Rejecting.", peer);
                    let _ = self.admin.respond_final(
                        channel_id,
                        Err(ActyxOSCode::ERR_UNAUTHORIZED
                            .with_message("Provided key is not authorized to access the API.")),
                    );
                    return;
                }

                self.enqueue(channel_id, payload);
            }
            libp2p_streaming_response::StreamingResponseEvent::<AdminProtocol>::CancelledRequest { .. } => {
                // all responses are one-shot at the moment, no need to cancel anything ongoing.
            }
            libp2p_streaming_response::StreamingResponseEvent::<AdminProtocol>::ResponseReceived { .. } => {}
            libp2p_streaming_response::StreamingResponseEvent::<AdminProtocol>::ResponseFinished { .. } => {}
        }
    }
}

impl NetworkBehaviourEventProcess<PingEvent> for ApiBehaviour {
    fn inject_event(&mut self, _event: PingEvent) {
        // ignored
    }
}

pub(crate) async fn mk_swarm(
    keypair: libp2p::core::identity::Keypair,
    node_tx: Sender<ExternalEvent>,
    bind_to: SocketAddrHelper,
    store: StoreTx,
    auth_info: Arc<Mutex<NodeApiSettings>>,
) -> anyhow::Result<(PeerId, WrappedBehaviour)> {
    let (peer_id, transport) = mk_transport(keypair).await?;
    let protocol = ApiBehaviour::new(node_tx, store, auth_info);

    let mut swarm = SwarmBuilder::new(transport, protocol, peer_id)
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();

    // Trying to bind to `/ip6/::0/tcp/0` (dual-stack) won't work, as
    // rust-libp2p sets `IPV6_V6ONLY` (or the platform equivalent) [0]. This is
    // why we have to to bind to ip4 and ip6 manually.
    // [0] https://github.com/libp2p/rust-libp2p/blob/master/transports/tcp/src/lib.rs#L322
    for addr in bind_to.to_multiaddrs() {
        debug!("Admin API trying to bind to {}", addr);
        Swarm::listen_on(&mut swarm, addr.clone()).with_context(|| {
            let port = addr
                .iter()
                .find_map(|x| match x {
                    Protocol::Tcp(p) => Some(p),
                    Protocol::Udp(p) => Some(p),
                    _ => None,
                })
                .unwrap_or_default();
            NodeErrorContext::BindFailed {
                port,
                component: "Admin".into(),
            }
        })?;
    }

    Ok((peer_id, swarm))
}

type TConnErr = libp2p::core::either::EitherError<
    // streaming-response logging
    libp2p::swarm::protocols_handler::ProtocolsHandlerUpgrErr<std::io::Error>,
    // ping
    libp2p::ping::handler::PingFailure,
>;

/// Wrapper object for driving the whole swarm
struct SwarmFuture(WrappedBehaviour);
impl SwarmFuture {
    pub(crate) fn swarm(&mut self) -> &mut WrappedBehaviour {
        &mut self.0
    }

    /// Poll the swarm once
    pub(crate) fn poll_swarm(&mut self, cx: &mut task::Context) -> futures::task::Poll<SwarmEvent<(), TConnErr>> {
        let fut = self.swarm().next_event();
        pin_mut!(fut);
        fut.poll(cx)
    }
}

impl Future for SwarmFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut task::Context) -> Poll<Self::Output> {
        // poll the swarm until pending
        while let Poll::Ready(event) = self.poll_swarm(cx) {
            match event {
                SwarmEvent::NewListenAddr(addr) => {
                    tracing::info!(target: "ADMIN_API_BOUND", "Admin API bound to {}.", addr);
                    self.0.behaviour_mut().state.admin_sockets.insert(addr);
                }
                SwarmEvent::ListenerError { error } => {
                    error!("SwarmEvent::ListenerError {}", error)
                }
                SwarmEvent::ListenerClosed {
                    reason: Err(error),
                    addresses,
                } => {
                    error!("SwarmEvent::ListenerClosed {} for {:?}", error, addresses);
                    for addr in addresses {
                        self.0.behaviour_mut().state.admin_sockets.remove(&addr);
                    }
                }
                o => {
                    debug!("Other swarm event {:?}", o);
                }
            }
        }

        Poll::Pending
    }
}
pub(crate) async fn start(swarm: WrappedBehaviour) {
    SwarmFuture(swarm).await;
}

async fn mk_transport(id_keys: identity::Keypair) -> anyhow::Result<(PeerId, Boxed<(PeerId, StreamMuxerBox)>)> {
    let peer_id = id_keys.public().into_peer_id();
    let transport = swarm::transport::build_transport(id_keys, None, Duration::from_secs(20))
        .await
        .context("Building libp2p transport")?;
    Ok((peer_id, transport))
}
