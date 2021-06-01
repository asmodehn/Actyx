use actyxos_sdk::{
    service::{EventResponse, EventService, PublishEvent, PublishRequest, SubscribeRequest, SubscribeResponse},
    Offset, TagSet, Url,
};
use actyxos_sdk::{tags, Payload};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use netsim_embed::unshare_user;
use quickcheck::{QuickCheck, TestResult};
use std::{convert::TryFrom, time::Duration};
use swarm_cli::Event;
use swarm_harness::{api::ApiClient, util::app_manifest, HarnessOpts};

#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    util::setup_logger();
    unshare_user()?;
    let res = QuickCheck::new()
        .tests(2)
        .quicktest(stress_single_store as fn(u8, u8, u8, u8) -> TestResult);
    if let Err(e) = res {
        if e.is_failure() {
            panic!("{:?}", e);
        }
    }
    Ok(())
}

fn stress_single_store(
    concurrent_publishes: u8,
    publish_chunk_size: u8,
    publish_chunks_per_client: u8,
    concurrent_subscribes: u8,
) -> TestResult {
    let concurrent_publishes = (concurrent_publishes >> 4).max(1);
    let publish_chunks_per_client = (publish_chunks_per_client >> 2).max(1);
    let concurrent_subscribes = (concurrent_subscribes >> 4).max(1);
    let publish_chunk_size = publish_chunk_size.max(1);

    let opts = HarnessOpts {
        n_nodes: 1,
        n_bootstrap: 0,
        delay_ms: 0,
        enable_mdns: false,
        enable_fast_path: true,
        enable_slow_path: true,
        enable_root_map: true,
        enable_discovery: true,
        enable_metrics: true,
        enable_api: Some("0.0.0.0:30001".parse().unwrap()),
    };

    let t = swarm_harness::run_netsim::<_, _, Event>(opts, move |mut sim| async move {
        tracing::info!(
            "running {}/{}/{}/{}",
            concurrent_publishes,
            publish_chunk_size,
            publish_chunks_per_client,
            concurrent_subscribes
        );
        let max_offset = Offset::try_from(
            (concurrent_publishes as u32 * publish_chunk_size as u32 * publish_chunks_per_client as u32) - 1,
        )
        .unwrap();

        let machine = &mut sim.machines_mut()[0];
        machine.send(swarm_cli::Command::ApiPort);
        let api_port = machine
            .select(|ev| swarm_harness::m!(ev, Event::ApiPort(port) => *port))
            .await
            .ok_or_else(|| anyhow::anyhow!("machine died"))?
            .ok_or_else(|| anyhow::anyhow!("api endpoint not configured"))?;

        let origin = Url::parse(&*format!("http://{}:{}", machine.addr(), api_port))?;
        let namespace = machine.namespace();

        let publish_clients = (0..concurrent_publishes)
            .map(|_| ApiClient::new(origin.clone(), app_manifest(), namespace))
            .collect::<Vec<_>>();

        let subscription_clients = (0..concurrent_subscribes)
            .map(|_| ApiClient::new(origin.clone(), app_manifest(), namespace))
            .collect::<Vec<_>>();

        let stream_0 = publish_clients[0].node_id().await?.node_id.stream(0.into());

        let mut futs = publish_clients
            .iter()
            .enumerate()
            .map(|(i, client)| {
                async move {
                    let tags = (0..publish_chunk_size).map(|_| tags!("my_test")).collect::<Vec<_>>();
                    let events = to_events(tags.clone());
                    for c in 0..publish_chunks_per_client {
                        tracing::debug!(
                            "Client {}/{}: Chunk {}/{} (chunk size {})",
                            i + 1,
                            concurrent_publishes,
                            c + 1,
                            publish_chunks_per_client,
                            publish_chunk_size,
                        );
                        let _meta = client.publish(to_publish(events.clone())).await?;
                    }
                    Result::<_, anyhow::Error>::Ok(())
                }
                .boxed()
            })
            .collect::<FuturesUnordered<_>>();

        let request = SubscribeRequest {
            offsets: None,
            query: "FROM 'my_test'".parse().unwrap(),
        };
        for client in subscription_clients {
            let request = request.clone();
            futs.push(
                async move {
                    client
                        .subscribe(request)
                        .then(move |req| async move {
                            let mut req = req?;
                            while let Some(x) = tokio::time::timeout(Duration::from_secs(10), req.next()).await? {
                                let SubscribeResponse::Event(EventResponse { offset, .. }) = x;
                                if offset >= max_offset {
                                    return Ok(());
                                }
                            }
                            anyhow::bail!("Stream ended")
                        })
                        .await
                }
                .boxed(),
            )
        }

        while let Some(res) = futs.next().await {
            if let Err(e) = res {
                anyhow::bail!("{:#}", e);
            }
        }

        let present = publish_clients[0].offsets().await?;
        let actual = present.present.get(stream_0);
        if actual != Some(max_offset) {
            anyhow::bail!("{:?} != {:?}", actual, max_offset)
        } else {
            Ok(())
        }
    });
    match t {
        Ok(()) => TestResult::passed(),
        Err(e) => {
            tracing::error!("Error from run: {:#?}", e);
            TestResult::error(format!("{:#?}", e))
        }
    }
}
fn to_events(tags: Vec<TagSet>) -> Vec<(TagSet, Payload)> {
    tags.into_iter().map(|t| (t, Payload::empty())).collect()
}
fn to_publish(events: Vec<(TagSet, Payload)>) -> PublishRequest {
    PublishRequest {
        data: events
            .into_iter()
            .map(|(tags, payload)| PublishEvent { tags, payload })
            .collect(),
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {}