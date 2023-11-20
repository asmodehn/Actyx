mod subscribe;
mod subscribe_monotonic;

use crate::cmd::AxCliCommand;
use futures::Future;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(version = crate::util::version::VERSION.as_str())]
/// interact with the event API through the admin port
pub enum EventsOpts {
    #[structopt(no_version)]
    Subscribe(subscribe::SubscribeOpts),
    #[structopt(no_version)]
    SubscribeMonotonic(subscribe_monotonic::SubscribeMonotonicOpts),
}

pub fn run(opts: EventsOpts, json: bool) -> Box<dyn Future<Output = ()> + Unpin> {
    match opts {
        EventsOpts::Subscribe(opt) => subscribe::EventsSubscribe::output(opt, json),
        EventsOpts::SubscribeMonotonic(opt) => subscribe_monotonic::EventsSubscribeMonotonic::output(opt, json),
    }
}
