use actyxos_sdk::Payload;
use actyxos_sdk::{
    language::{Query, TagAtom, TagExpr},
    service::{EventService, PublishEvent, PublishRequest, SubscribeRequest},
    Tag, TagSet,
};
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use netsim_embed::unshare_user;
use quickcheck::{Arbitrary, Gen, QuickCheck, TestResult};
use std::{collections::BTreeMap, str::FromStr};
use swarm_cli::Event;
use swarm_harness::api::ApiClient;
use swarm_harness::util::app_manifest;
use swarm_harness::HarnessOpts;

const MAX_NODES: usize = 20;
#[cfg(target_os = "linux")]
fn main() -> anyhow::Result<()> {
    util::setup_logger();
    unshare_user()?;

    let res = QuickCheck::new()
        .tests(5)
        .gen(Gen::new(200))
        .quicktest(interleaved as fn(TestInput) -> TestResult);
    if let Err(e) = res {
        panic!("{:?}", e);
    }

    Ok(())
}

#[derive(Clone, Debug)]
enum TestCommand {
    Subscribe {
        tags: TagSet,
        node: usize, // index into nodes array
    },
    Publish {
        node: usize, // index into nodes array
        tags: Vec<TagSet>,
    },
}

#[derive(Clone, Debug)]
struct TestInput {
    n_nodes: usize,
    commands: Vec<TestCommand>,
    cnt_per_tagset: BTreeMap<TagSet, usize>,
}
fn to_query(tags: TagSet) -> Query {
    let from = tags
        .iter()
        .map(TagAtom::Tag)
        .map(TagExpr::Atom)
        .reduce(|a, b| a.and(b))
        .unwrap_or(TagExpr::Atom(TagAtom::AllEvents));
    Query { from, ops: vec![] }
}
fn cnt_per_tag(cmds: &[TestCommand]) -> BTreeMap<TagSet, usize> {
    let mut map: BTreeMap<TagSet, usize> = Default::default();
    for c in cmds {
        if let TestCommand::Publish { tags, .. } = c {
            for t in tags {
                *map.entry(t.clone()).or_default() += 1;
            }
        }
    }
    map
}
impl Arbitrary for TestInput {
    fn arbitrary(g: &mut Gen) -> Self {
        let n = (Vec::<bool>::arbitrary(g).len() % MAX_NODES).max(1); // 0 < nodes <= MAX_NODES
        let nodes: Vec<usize> = (0..n).into_iter().enumerate().map(|(i, _)| i).collect();
        // fancy tagset don't really matter here
        let possible_tagsets = Vec::<Vec<bool>>::arbitrary(g)
            .into_iter()
            .enumerate()
            .map(|(idx, v)| {
                v.into_iter()
                    .enumerate()
                    .map(|(idx2, _)| Tag::from_str(&*format!("{}-{}", idx, idx2)).unwrap())
                    .collect::<TagSet>()
            })
            .collect::<Vec<_>>();
        let commands = Vec::<bool>::arbitrary(g)
            .into_iter()
            .map(|_| {
                match g.choose(&[0, 1, 2]).unwrap() {
                    1 => TestCommand::Subscribe {
                        tags: g.choose(&possible_tagsets[..]).cloned().unwrap_or_default(),
                        node: *g.choose(&nodes[..]).unwrap(),
                    },
                    _ => {
                        let tags = possible_tagsets
                            .iter()
                            .filter(|_| bool::arbitrary(g))
                            .cloned()
                            .collect();
                        TestCommand::Publish {
                            tags,
                            node: *g.choose(&nodes[..]).unwrap(), // stream: possible_streams,
                        }
                    }
                }
            })
            .collect::<Vec<_>>();

        Self {
            cnt_per_tagset: cnt_per_tag(&commands),
            commands,
            n_nodes: nodes.len(),
        }
    }
    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(TestShrinker::new(self.clone()))
    }
}
enum ShrinkState {
    ShrinkNodes,
    ShrinkCommands,
}
struct TestShrinker {
    seed: TestInput,
    last: TestInput,
    state: ShrinkState,
}
impl TestShrinker {
    fn new(seed: TestInput) -> Self {
        Self {
            last: seed.clone(),
            seed,
            state: ShrinkState::ShrinkNodes,
        }
    }
}
impl Iterator for TestShrinker {
    type Item = TestInput;
    fn next(&mut self) -> Option<Self::Item> {
        tracing::info!("Shrinking from {}/{}", self.seed.n_nodes, self.seed.commands.len());
        loop {
            match &mut self.state {
                ShrinkState::ShrinkNodes => {
                    if self.last.n_nodes > 1 {
                        // Try with less nodes
                        self.last.n_nodes /= 2;
                        break Some(self.last.clone());
                    } else {
                        // less nodes didn't work :-(
                        self.last = self.seed.clone();
                        self.state = ShrinkState::ShrinkCommands;
                    }
                }
                ShrinkState::ShrinkCommands => {
                    if self.last.commands.len() > 2 {
                        let len = self.last.commands.len();
                        self.last.commands.drain(len - 2..len);
                        self.last.cnt_per_tagset = cnt_per_tag(&self.last.commands);
                        break Some(self.last.clone());
                    } else {
                        // less commands didn't work :-(
                        // give up
                        break None;
                    }
                }
            }
        }
    }
}

fn interleaved(input: TestInput) -> TestResult {
    let TestInput {
        commands,
        n_nodes,
        cnt_per_tagset,
    } = input;
    tracing::info!("{} nodes with {} commands", n_nodes, commands.len(),);
    let opts = HarnessOpts {
        n_nodes,
        n_bootstrap: 1,
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
        let machines = sim.machines().iter().map(|m| m.id()).collect::<Vec<_>>();
        assert_eq!(machines.len(), n_nodes);
        let mut futs = commands
            .into_iter()
            .enumerate()
            .map(|(cmd_id, cmd)| match cmd {
                TestCommand::Publish { tags, node } => {
                    let id = machines[node % n_nodes];
                    let client = ApiClient::from_machine(sim.machine(id), app_manifest()).unwrap();

                    let events = to_events(tags.clone());

                    tracing::debug!("Cmd {} / Node {}: Publishing {} events", cmd_id, node, events.len());
                    async move {
                        client.publish(to_publish(events.clone())).await?;
                        Result::<_, anyhow::Error>::Ok(())
                    }
                    .boxed()
                }
                TestCommand::Subscribe { node, tags, .. } => {
                    let expected_cnt = *cnt_per_tagset.get(&tags).unwrap_or(&0);
                    tracing::debug!(
                        "Cmd {} / Node {}: subscribing, expecting {} events",
                        cmd_id,
                        node,
                        expected_cnt
                    );

                    let id = machines[node % n_nodes];
                    let client = ApiClient::from_machine(sim.machine(id), app_manifest()).unwrap();
                    let query = to_query(tags.clone());
                    let request = SubscribeRequest { offsets: None, query };
                    async move {
                        let mut req = client.subscribe(request).await?;
                        let mut actual = 0;
                        if expected_cnt > 0 {
                            while req.next().await.is_some() {
                                actual += 1;
                                tracing::debug!("Cmd {} / Node {}: {}/{}", cmd_id, node, actual, expected_cnt,);
                                if actual >= expected_cnt {
                                    tracing::debug!("Cmd {} / Node {}: Done", cmd_id, node);
                                    break;
                                }
                            }
                        }
                        Result::<_, anyhow::Error>::Ok(())
                    }
                }
                .boxed(),
            })
            .collect::<FuturesUnordered<_>>();

        while let Some(res) = futs.next().await {
            res?;
        }
        Ok(())
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
