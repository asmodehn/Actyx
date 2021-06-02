mod inspect;
mod ls;
use crate::cmd::AxCliCommand;
use futures::Future;
use inspect::InspectOpts;
use ls::LsOpts;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
/// Manage nodes
pub enum NodesOpts {
    #[structopt(name = "ls")]
    /// Show node info and status
    Ls(LsOpts),
    #[structopt(name = "inspect")]
    Inspect(InspectOpts),
}

pub fn run(opts: NodesOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        NodesOpts::Ls(opt) => ls::NodesLs::output(opt, json),
        NodesOpts::Inspect(opt) => inspect::NodesInspect::output(opt, json),
    }
}
