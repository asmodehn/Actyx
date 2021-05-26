#![cfg(target_os = "linux")]

use anyhow::Result;
use futures::prelude::*;
use netsim_embed::{DelayBuffer, Ipv4Range, Netsim};
use std::time::Duration;
use structopt::StructOpt;
use swarm_cli::{multiaddr, Command, Config, Event, Multiaddr, PeerId};
use tempdir::TempDir;

#[derive(StructOpt)]
pub struct HarnessOpts {
    #[structopt(long, default_value = "2")]
    pub n_nodes: usize,

    #[structopt(long, default_value = "0")]
    pub delay_ms: u64,

    #[structopt(long, default_value = "0")]
    pub n_bootstrap: usize,

    #[structopt(long)]
    pub enable_mdns: bool,

    #[structopt(long)]
    pub enable_fast_path: bool,

    #[structopt(long)]
    pub enable_slow_path: bool,

    #[structopt(long)]
    pub enable_root_map: bool,

    #[structopt(long)]
    pub enable_discovery: bool,

    #[structopt(long)]
    pub enable_metrics: bool,
}

pub trait MachineExt {
    fn peer_id(&self) -> PeerId;
    fn multiaddr(&self) -> Multiaddr;
}

impl MachineExt for netsim_embed::Machine<Command, Event> {
    fn peer_id(&self) -> PeerId {
        swarm_cli::keypair(self.id().0 as u64).into()
    }

    fn multiaddr(&self) -> Multiaddr {
        format!("/ip4/{}/tcp/30000", self.addr()).parse().unwrap()
    }
}

pub trait MultiaddrExt {
    fn is_loopback(&self) -> bool;
}

impl MultiaddrExt for Multiaddr {
    fn is_loopback(&self) -> bool {
        if let Some(multiaddr::Protocol::Ip4(addr)) = self.iter().next() {
            if !addr.is_loopback() {
                return false;
            }
        }
        true
    }
}

pub fn run_netsim<F, F2>(mut f: F) -> Result<()>
where
    F: FnMut(Netsim<Command, Event>) -> F2,
    F2: Future<Output = Result<()>> + Send,
{
    util::setup_logger();
    let opts = HarnessOpts::from_args();
    let temp_dir = TempDir::new("swarm-harness")?;
    netsim_embed::unshare_user()?;
    async_global_executor::block_on(async move {
        let mut sim = Netsim::new();
        let net = sim.spawn_network(Ipv4Range::random_local_subnet());
        let mut addrs = Vec::with_capacity(opts.n_bootstrap);
        let mut bootstrap = Vec::with_capacity(opts.n_bootstrap);
        for i in 0..opts.n_bootstrap {
            let peer_id: PeerId = swarm_cli::keypair(i as u64).into();
            let addr = sim.network(net).random_addr();
            let maddr = format!("/ip4/{}/tcp/30000/p2p/{}", addr, peer_id);
            addrs.push(addr);
            bootstrap.push(maddr.parse().unwrap());
        }
        for i in 0..opts.n_nodes {
            let cfg = Config {
                path: Some(temp_dir.path().join(i.to_string())),
                node_name: None,
                keypair: i as _,
                listen_on: vec!["/ip4/0.0.0.0/tcp/30000".parse().unwrap()],
                bootstrap: bootstrap.clone(),
                external: vec![],
                enable_mdns: opts.enable_mdns,
                enable_fast_path: opts.enable_fast_path,
                enable_slow_path: opts.enable_slow_path,
                enable_root_map: opts.enable_root_map,
                enable_discovery: opts.enable_discovery,
                enable_metrics: opts.enable_metrics,
            };
            let mut delay = DelayBuffer::new();
            delay.set_delay(Duration::from_millis(opts.delay_ms));
            let machine = sim.spawn_machine(cfg.into(), Some(delay)).await;
            sim.plug(machine, net, addrs.get(i).copied()).await;
        }
        f(sim).await
    })
}
