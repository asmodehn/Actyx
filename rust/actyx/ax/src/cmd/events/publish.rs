use crate::cmd::{AxCliCommand, ConsoleOpt};
use actyx_sdk::{
    service::{PublishEvent, PublishRequest, PublishResponse},
    Payload, Tag, TagSet,
};
use chrono::{DateTime, Utc};
use futures::{future::ready, Stream, StreamExt};
use genawaiter::sync::Co;
use structopt::StructOpt;
use util::{
    formats::{
        events_protocol::{EventsRequest, EventsResponse},
        ActyxOSCode, ActyxOSError, ActyxOSResult,
    },
    gen_stream::GenStream,
};

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
/// publish an event
pub struct PublishOpts {
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
    /// event payload (JSON)
    payload: serde_json::Value,
    #[structopt(long, short)]
    /// tag (can be given multiple times)
    tag: Option<Vec<Tag>>,
}

pub struct EventsPublish;
impl AxCliCommand for EventsPublish {
    type Opt = PublishOpts;
    type Output = PublishResponse;

    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        Box::new(
            GenStream::new(move |co: Co<_>| async move {
                let tags = opts.tag.unwrap_or_default().into_iter().collect::<TagSet>();
                let payload = Payload::from_json_value(opts.payload)
                    .map_err(|msg| ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, msg))?;

                let mut conn = opts.console_opt.connect().await?;
                let mut s = conn
                    .request_events(EventsRequest::Publish(PublishRequest {
                        data: vec![PublishEvent { tags, payload }],
                    }))
                    .await?;

                while let Some(x) = s.next().await {
                    match x {
                        EventsResponse::Publish(res) => co.yield_(Ok(Some(res))).await,
                        EventsResponse::Error { message } => {
                            co.yield_(Err(ActyxOSError::new(ActyxOSCode::ERR_INVALID_INPUT, message)))
                                .await
                        }
                        _ => {}
                    }
                }
                Ok(None)
            })
            .filter_map(|x| ready(x.transpose())),
        )
    }

    fn pretty(result: Self::Output) -> String {
        use std::fmt::Write;

        let mut s = String::new();
        for key in result.data {
            let ts = DateTime::<Utc>::from(key.timestamp);
            writeln!(&mut s, "published event {}/{} at {}", key.stream, key.offset, ts).unwrap();
        }
        s
    }
}