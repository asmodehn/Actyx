use crate::cmd::{formats::Result, AxCliCommand, ConsoleOpt};
use futures::{stream, Stream, TryFutureExt};
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use structopt::StructOpt;
use util::formats::{ActyxOSError, ActyxOSResult, AdminRequest, AdminResponse};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    scope: String,
}

pub struct SettingsUnset();
impl AxCliCommand for SettingsUnset {
    type Opt = UnsetOpt;
    type Output = Output;
    fn run(opts: Self::Opt) -> Box<dyn Stream<Item = ActyxOSResult<Self::Output>> + Unpin> {
        let r = Box::pin(run(opts).map_err(Into::into));
        Box::new(stream::once(r))
    }
    fn pretty(result: Self::Output) -> String {
        format!("Successfully unset settings at {}.", result.scope)
    }
}
#[derive(Serialize)]
struct RequestBody {
    scope: settings::Scope,
}

#[derive(StructOpt, Debug)]
#[structopt(version = env!("AX_CLI_VERSION"))]
pub struct UnsetOpt {
    #[structopt(flatten)]
    actual_opts: UnsetSettingsCommand,
    #[structopt(flatten)]
    console_opt: ConsoleOpt,
}

#[derive(StructOpt, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnsetSettingsCommand {
    #[structopt(name = "SCOPE", parse(try_from_str = super::parse_scope))]
    /// Scope for which you want to unset the settings; use `/` for the root scope.
    scope: settings::Scope,
}

pub async fn run(mut opts: UnsetOpt) -> Result<Output> {
    let scope = opts.actual_opts.scope.clone();

    match opts
        .console_opt
        .authority
        .request(
            &opts.console_opt.identity.try_into()?,
            AdminRequest::SettingsUnset {
                scope: opts.actual_opts.scope,
            },
        )
        .await
    {
        Ok(AdminResponse::SettingsUnsetResponse) => Ok(Output {
            scope: super::print_scope(scope),
        }),
        Ok(r) => Err(ActyxOSError::internal(format!("Unexpected reply: {:?}", r))),
        Err(err) => Err(err),
    }
}
