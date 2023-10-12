#![doc = include_str!("../README.md")]
#![doc(html_logo_url = "https://developer.actyx.com/img/logo.svg")]
#![doc(html_favicon_url = "https://developer.actyx.com/img/favicon.ico")]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::inconsistent_digit_grouping)]

#[macro_use]
#[cfg(test)]
extern crate serde_json;
#[allow(unused_imports)]
#[macro_use]
extern crate actyx_sdk_macros;
#[doc(hidden)]
pub use actyx_sdk_macros::*;

#[macro_use]
mod scalar;

mod app_manifest;
#[cfg(any(test, feature = "arb"))]
pub mod arb;
#[cfg(feature = "client")]
mod client;
mod event;
pub mod language;
pub mod legacy;
mod offset;
mod scalars;
pub mod service;
mod tags;
mod timestamp;
pub mod types;

pub use app_manifest::AppManifest;
#[cfg(feature = "client")]
pub use client::ActyxClient;
pub use event::{Event, EventKey, Metadata, Opaque, Payload};
pub use offset::{Offset, OffsetError, OffsetMap, OffsetOrMin};
pub use scalars::{AppId, NodeId, StreamId, StreamNr};
pub use tags::{Tag, TagSet};
pub use timestamp::{LamportTimestamp, Timestamp};
#[cfg(feature = "client")]
pub use url::Url;

#[cfg(test)]
mod test_util;

#[cfg(test)]
pub use test_util::*;

use derive_more::Display;
#[derive(Debug, Display, PartialEq, Eq)]
pub enum ParseError {
    #[display(fmt = "Empty string is not permissible for Tag")]
    EmptyTag,
    #[display(fmt = "Empty string is not permissible for AppId")]
    EmptyAppId,
    #[display(fmt = "Invalid AppId: '{}'", _0)]
    InvalidAppId(String),
}
impl std::error::Error for ParseError {}
