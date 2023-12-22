use super::{Authority, AxCliCommand};
use ax_core::{
    node_connection::{connect, mk_swarm, request_single, Task},
    private_key::{AxPrivateKey, KeyPathWrapper},
    util::formats::{ActyxOSCode, ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse, TopicLsResponse},
};
use ax_sdk::types::NodeId;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Cell, Table};
use futures::{channel::mpsc, future::join_all, stream};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "connection")]
pub enum LsOutput {
    Reachable { host: String, response: TopicLsResponse },
    Unreachable { host: String },
    Unauthorized { host: String },
    Error { host: String, error: ActyxOSError },
}

// This code is mostly duplicated from `ax/src/cmd/nodes/ls.rs`
async fn request(timeout: u8, mut conn: mpsc::Sender<Task>, authority: Authority) -> LsOutput {
    let host = authority.original.clone();
    let response = tokio::time::timeout(Duration::from_secs(timeout.into()), async move {
        let peer = connect(&mut conn, authority).await?;
        request_single(&mut conn, move |tx| Task::Admin(peer, AdminRequest::TopicLs, tx), Ok).await
    })
    .await;
    if let Ok(response) = response {
        match response {
            Ok(AdminResponse::TopicLsResponse(response)) => LsOutput::Reachable { host, response },
            Ok(response) => LsOutput::Error {
                host,
                error: ActyxOSError::internal(format!("Unexpected response from node: {:?}", response)),
            },
            Err(error) if error.code() == ActyxOSCode::ERR_NODE_UNREACHABLE => LsOutput::Unreachable { host },
            Err(error) if error.code() == ActyxOSCode::ERR_UNAUTHORIZED => LsOutput::Unauthorized { host },
            Err(error) => LsOutput::Error { host, error },
        }
    } else {
        // The difference between this unreachable and the previous lies on the timeout
        // here `ax` is "giving up" and on the previous, the node is actually unreachable
        LsOutput::Error {
            host,
            error: ActyxOSError::new(ax_core::util::formats::ActyxOSCode::ERR_NODE_UNREACHABLE, "timeout"),
        }
    }
}

async fn ls_run(opts: LsOpts) -> ActyxOSResult<Vec<LsOutput>> {
    // Get the auth and timeout parameters
    let identity: AxPrivateKey = (&opts.identity).try_into()?;
    let timeout = opts.timeout;
    // Get a communication channel to the swarm
    let (task, channel) = mk_swarm(identity).await?;
    tokio::spawn(task);
    // Send the ls command to all nodes in authority and return the results
    Ok(join_all(
        opts.authority
            .into_iter()
            .map(|a| request(timeout, channel.clone(), a))
            .collect::<Vec<_>>(),
    )
    .await)
}

pub struct TopicsList;

impl AxCliCommand for TopicsList {
    type Opt = LsOpts;

    type Output = Vec<LsOutput>;

    fn run(
        opts: Self::Opt,
    ) -> Box<dyn futures::Stream<Item = ax_core::util::formats::ActyxOSResult<Self::Output>> + Unpin> {
        let requests = Box::pin(ls_run(opts));
        Box::new(stream::once(requests))
    }

    fn pretty(result: Self::Output) -> String {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL_CONDENSED)
            .set_header(["NODE ID", "HOST", "TOPIC", "SIZE", "ACTIVE"]);

        let mut last: Option<NodeId> = None;
        for output in result {
            match output {
                LsOutput::Reachable { host, response } => {
                    for (topic_name, topic_size) in response.topics {
                        let active = if response.active_topic == topic_name { "*" } else { "" };
                        match last {
                            Some(last_node_id) if last_node_id == response.node_id => {
                                table.add_row([
                                    Cell::new(""),
                                    Cell::new(""),
                                    Cell::new(topic_name),
                                    Cell::new(topic_size),
                                    Cell::new(active),
                                ]);
                            }
                            _ => {
                                table.add_row([
                                    Cell::new(response.node_id),
                                    Cell::new(&host),
                                    Cell::new(topic_name),
                                    Cell::new(topic_size),
                                    Cell::new(active),
                                ]);
                                last = Some(response.node_id);
                            }
                        }
                    }
                }
                LsOutput::Unreachable { host } => {
                    table.add_row([Cell::new("AX was unreachable on host"), Cell::new(host)]);
                }
                LsOutput::Unauthorized { host } => {
                    table.add_row([Cell::new("Unauthorized on host"), Cell::new(host)]);
                }
                LsOutput::Error { host, error } => {
                    table.add_row([
                        Cell::new(format!("Received error \"{}\" from host", error)),
                        Cell::new(host),
                    ]);
                }
            }
        }
        table.to_string()
    }
}

/// List all topics
#[derive(clap::Parser, Clone, Debug)]
pub struct LsOpts {
    /// The IP addresses or `<host>:<admin port>` of the target nodes.
    #[arg(name = "NODE", required = true)]
    authority: Vec<Authority>,
    /// The private key file to use for authentication.
    #[arg(short, long)]
    identity: Option<KeyPathWrapper>,
    /// Timeout time for the operation (in seconds, with a maximum of 255).
    #[arg(short, long, default_value = "5")]
    timeout: u8,
}
