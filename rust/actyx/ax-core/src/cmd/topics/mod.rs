mod delete;
mod ls;

use futures::Future;

use self::{
    delete::{DeleteOpts, TopicsDelete},
    ls::{LsOpts, TopicsList},
};

use super::{Authority, AxCliCommand, KeyPathWrapper};

/// manage topics
#[derive(clap::Parser, Clone, Debug)]
pub enum TopicsOpts {
    Ls(LsOpts),
    Delete(DeleteOpts),
}

pub fn run(opts: TopicsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        TopicsOpts::Ls(opts) => TopicsList::output(opts, json),
        TopicsOpts::Delete(opts) => TopicsDelete::output(opts, json),
    }
}
