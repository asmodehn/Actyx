use actyxos_sdk::{tags, Payload};
use anyhow::Result;
use swarm_cli::{Command, Event};

fn main() -> Result<()> {
    swarm_harness::run_netsim(|mut network| async move {
        network
            .machine(0)
            .send(Command::Append(
                0.into(),
                vec![(tags!("a"), Payload::from_json_str("\"hello world\"").unwrap())],
            ))
            .await;
        for machine in &mut network.machines_mut()[1..] {
            machine.send(Command::Query("'a'".parse().unwrap())).await;
        }
        for machine in &mut network.machines_mut()[1..] {
            if let Some(Event::Result(ev)) = machine.recv().await {
                println!("{:?}", ev);
            }
        }
        network
    })
}
