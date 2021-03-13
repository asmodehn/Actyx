/*
 * Copyright 2021 Actyx AG
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
//! [ActyxOS](https://developer.actyx.com/docs/os/introduction) makes it easy to run distributed
//! applications on multiple nodes. It is a piece of software that allows you to run your own apps
//! on one or more edge devices and have these apps seamlessly communicate and share data with
//! each other.
//!
//! This crate defines the data types needed for communicating with ActyxOS and provides Rust
//! bindings for the ActyxOS APIs. It also provides serialization instances for processing the
//! events with [`differential-dataflow`](https://docs.rs/differential-dataflow) under the `"dataflow"`
//! [feature flag](#feature-flags).
//!
//! # Examples
//!
//! Below you find a full example using the [`EventService`](event_service/struct.EventService.html)
//! client that retrieves some events. Please adapt the `semantics` to match your stored events
//! in order to see output.
//!
//! > _Note: this example needs the `client` feature to compile_
//!
//! ```no_run
//! use actyxos_sdk::{
//!   service::{EventService, Order, QueryRequest, QueryResponse},
//!   http_client::HttpClient,
//! };
//! use futures::stream::StreamExt;
//!
//! #[tokio::main]
//! pub async fn main() -> anyhow::Result<()> {
//!   // client for locally running Actyx Event Service
//!   let service = HttpClient::default().await?;
//!
//!   // retrieve largest currently known event stream cursor
//!   let offsets = service.offsets().await?;
//!
//!   // all events matching the given subscription
//!   // sorted backwards, i.e. youngest to oldest
//!   let mut events = service
//!       .query(QueryRequest {
//!           lower_bound: None,
//!           upper_bound: offsets,
//!           r#where: "MyFish".parse()?,
//!           order: Order::Desc,
//!       })
//!       .await?;
//!
//!   // print out the payload of each event
//!   // (cf. Payload::extract for more options)
//!   while let Some(QueryResponse::Event(event)) = events.next().await {
//!       println!("{}", event.payload.json_value());
//!   }
//!   Ok(())
//! }
//! ```
//!
//! # Feature flags
//!
//! The default is to provide only the data types with serialization and deserialization support
//! for [`serde`](https://docs.rs/serde). The following features can be enabled in addition:
//!
//! - `client`: include HTTP client bindings using the [`reqwest`](https://docs.rs/reqwest) crate
//! - `dataflow`: provide [`Abomonation`](https://docs.rs/abomonation) instances for use with tools
//!   like [`Differential Dataflow`](https://docs.rs/differential-dataflow)
#![doc(html_logo_url = "https://developer.actyx.com/img/logo.svg")]
#![doc(html_favicon_url = "https://developer.actyx.com/img/favicon.ico")]
#![allow(clippy::unreadable_literal)]
#![allow(clippy::inconsistent_digit_grouping)]

#[macro_use]
#[cfg(feature = "dataflow")]
extern crate abomonation_derive;
#[macro_use]
#[cfg(test)]
extern crate serde_json;
#[allow(unused_imports)]
#[macro_use]
extern crate actyxos_sdk_macros;
#[doc(hidden)]
pub use actyxos_sdk_macros::*;

#[macro_use]
mod scalar;

#[cfg(any(test, feature = "arb"))]
pub mod arb;
pub mod event;
pub mod expression;
#[cfg(feature = "client")]
pub mod http_client;
pub mod legacy;
pub mod offset;
pub mod scalars;
pub mod service;
pub mod tags;
pub mod timestamp;
pub mod types;

pub use event::{Event, EventKey, Metadata, Opaque, Payload};
pub use expression::{Dnf, Expression};
#[cfg(feature = "client")]
pub use http_client::HttpClient;
pub use offset::{Offset, OffsetMap, OffsetOrMin};
pub use scalars::{AppId, NodeId, StreamId, StreamNr};
pub use service::EventService;
pub use tags::{Tag, TagSet};
pub use timestamp::{LamportTimestamp, Timestamp};

#[cfg(test)]
mod test_util;

#[cfg(test)]
pub use test_util::*;
