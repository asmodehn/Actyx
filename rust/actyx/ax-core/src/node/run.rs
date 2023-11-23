use super::BindToOpts;
use anyhow::Result;
use derive_more::{Display, Error};
use std::{path::PathBuf, str::FromStr};
use structopt::StructOpt;

#[derive(Debug)]
pub enum Color {
    Off,
    Auto,
    On,
}

impl FromStr for Color {
    type Err = NoColor;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "1" | "on" | "true" => Ok(Self::On),
            "0" | "off" | "false" => Ok(Self::Off),
            "auto" => Ok(Self::Auto),
            _ => Err(NoColor),
        }
    }
}

#[derive(Debug, Display, Error)]
#[display(fmt = "allowed values are 1, on, true, 0, off, false, auto (case insensitive)")]
pub struct NoColor;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "ax",
    about = "run the ax distributed event database",
    help_message = "Print help information (use --help for more details)",
    after_help = "For one-off log verbosity override, you may start with the environment variable \
        RUST_LOG set to “debug” or “node=debug,info” (the former logs all debug messages while \
        the latter logs at debug level for the “node” code module and info level for everything \
        else).
        ",
    rename_all = "kebab-case",
    version = crate::util::version::VERSION.as_str(),
)]
pub struct RunOpts {
    /// Path where to store all the data of the Actyx node.
    #[structopt(
        long,
        env = "ACTYX_PATH",
        long_help = "Path where to store all the data of the Actyx node. \
            Defaults to creating <current working dir>/actyx-data"
    )]
    pub working_dir: Option<PathBuf>,

    #[structopt(flatten)]
    pub bind_options: BindToOpts,

    #[structopt(short, long, hidden = true)]
    pub random: bool,

    /// This does not do anything; kept for backward-compatibility
    #[structopt(long)]
    pub background: bool,

    #[structopt(long)]
    pub version: bool,

    /// Control whether to use ANSI color sequences in log output.
    #[structopt(
        long,
        env = "ACTYX_COLOR",
        long_help = "Control whether to use ANSI color sequences in log output. \
            Valid values (case insensitive) are 1, true, on, 0, false, off, auto \
            (default is on, auto only uses colour when stderr is a terminal). \
            Defaults to 1."
    )]
    pub log_color: Option<Color>,

    /// Output logs as JSON objects (one per line)
    #[structopt(
        long,
        env = "ACTYX_LOG_JSON",
        long_help = "Output logs as JSON objects (one per line) if the value is \
            1, true, on or if stderr is not a terminal and the value is auto \
            (all case insensitive). Defaults to 0."
    )]
    pub log_json: Option<Color>,
}
