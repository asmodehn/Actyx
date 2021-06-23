use std::{convert::TryFrom, num::NonZeroU64};

use actyx_sdk::{
    language,
    service::{
        EventResponse, OffsetMapResponse, OffsetsResponse, Order, PublishEvent, PublishRequest, PublishResponse,
        PublishResponseKey, QueryRequest, QueryResponse, StartFrom, SubscribeMonotonicRequest,
        SubscribeMonotonicResponse, SubscribeRequest, SubscribeResponse,
    },
    AppId, Event, Metadata, OffsetOrMin, Payload,
};
use ax_futures_util::prelude::*;
use futures::{
    future,
    stream::{self, BoxStream, StreamExt},
};
use runtime::value::Value;
use swarm::event_store::{self, EventStore};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Store error while writing: {0}")]
    StoreWriteError(#[from] anyhow::Error),
    #[error("Store error while reading: {0}")]
    StoreReadError(#[from] event_store::Error),
}

#[derive(Clone)]
pub struct EventService {
    store: EventStore,
}

impl EventService {
    pub fn new(store: EventStore) -> EventService {
        EventService { store }
    }
}

impl EventService {
    pub async fn offsets(&self) -> anyhow::Result<OffsetsResponse> {
        let offsets = self.store.offsets().next().await.expect("offset stream stopped");
        let present = offsets.present();
        let to_replicate = offsets
            .replication_target()
            .stream_iter()
            .filter_map(|(stream, target)| {
                let actual = present.offset(stream);
                let diff = OffsetOrMin::from(target) - actual;
                u64::try_from(diff).ok().and_then(NonZeroU64::new).map(|o| (stream, o))
            })
            .collect();
        Ok(OffsetsResponse { present, to_replicate })
    }

    pub async fn publish(&self, app_id: AppId, request: PublishRequest) -> anyhow::Result<PublishResponse> {
        let events = request
            .data
            .into_iter()
            .map(|PublishEvent { tags, payload }| (tags, payload))
            .collect();
        let meta = self
            .store
            .persist(app_id, events)
            .await
            .map_err(Error::StoreWriteError)?;
        let response = PublishResponse {
            data: meta
                .into_iter()
                .map(|(lamport, offset, stream_nr, timestamp)| PublishResponseKey {
                    lamport,
                    offset,
                    stream: self.store.node_id().stream(stream_nr),
                    timestamp,
                })
                .collect(),
        };
        Ok(response)
    }

    pub async fn query(
        &self,
        _app_id: AppId,
        request: QueryRequest,
    ) -> anyhow::Result<BoxStream<'static, QueryResponse>> {
        let tag_expr = &request.query.from;
        let upper_bound = match request.upper_bound {
            Some(offsets) => offsets,
            None => self.store.present().await,
        };
        let stream = match request.order {
            Order::Asc => self
                .store
                .bounded_forward(tag_expr, request.lower_bound, upper_bound.clone())
                .await
                .map(|s| s.boxed()),
            Order::Desc => self
                .store
                .bounded_backward(tag_expr, request.lower_bound, upper_bound.clone())
                .await
                .map(|s| s.boxed()),
            Order::StreamAsc => self
                .store
                .bounded_forward_per_stream(tag_expr, request.lower_bound, upper_bound.clone())
                .await
                .map(|s| s.boxed()),
        };
        let response = stream
            .map_err(Error::StoreReadError)?
            .flat_map(mk_feed(request.query))
            .map(QueryResponse::Event)
            .chain(stream::once(future::ready(QueryResponse::Offsets(OffsetMapResponse {
                offsets: upper_bound,
            }))))
            .boxed();
        Ok(response)
    }

    pub async fn subscribe(
        &self,
        _app_id: AppId,
        request: SubscribeRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeResponse>> {
        let present = self.store.present().await;
        let bounded = self
            .store
            .bounded_forward(&request.query.from, request.lower_bound, present.clone())
            .await
            .map_err(Error::StoreReadError)?
            .flat_map(mk_feed(request.query.clone()))
            .map(SubscribeResponse::Event);
        let offsets = stream::once(future::ready(SubscribeResponse::Offsets(OffsetMapResponse {
            offsets: present.clone(),
        })));
        let unbounded = self
            .store
            .unbounded_forward_per_stream(&request.query.from, Some(present))
            .flat_map(mk_feed(request.query.clone()))
            .map(SubscribeResponse::Event);
        Ok(bounded.chain(offsets).chain(unbounded).boxed())
    }

    pub async fn subscribe_monotonic(
        &self,
        _app_id: AppId,
        request: SubscribeMonotonicRequest,
    ) -> anyhow::Result<BoxStream<'static, SubscribeMonotonicResponse>> {
        let present = self.store.present().await;
        let bounded = self
            .store
            .bounded_forward(&request.query.from, Some(request.from.min_offsets()), present.clone())
            .await
            .map_err(Error::StoreReadError)?
            .flat_map(mk_feed(request.query.clone()))
            .map(|event| SubscribeMonotonicResponse::Event { event, caught_up: true });
        let offsets = stream::once(future::ready(SubscribeMonotonicResponse::Offsets(OffsetMapResponse {
            offsets: present.clone(),
        })));
        let unbounded = self
            .store
            .unbounded_forward_per_stream(&request.query.from, Some(present))
            .flat_map({
                let feed = mk_feed(request.query.clone());
                let mut latest = match &request.from {
                    StartFrom::LowerBound(offsets) => self
                        .store
                        .bounded_backward(&request.query.from, None, offsets.clone())
                        .await
                        .map_err(Error::StoreReadError)?
                        .next()
                        .await
                        .map(|event| event.key),
                };
                move |e| {
                    let key = Some(e.key);
                    if key > latest {
                        latest = key;
                        feed(e)
                            .map(|event| SubscribeMonotonicResponse::Event { event, caught_up: true })
                            .left_stream()
                    } else {
                        stream::once(async move { SubscribeMonotonicResponse::TimeTravel { new_start: e.key } })
                            .right_stream()
                    }
                }
            })
            .take_until_condition(|e| future::ready(matches!(e, SubscribeMonotonicResponse::TimeTravel { .. })));
        Ok(bounded.chain(offsets).chain(unbounded).boxed())
    }
}

fn mk_feed(query: language::Query) -> impl Fn(Event<Payload>) -> BoxStream<'static, EventResponse<Payload>> {
    let query = runtime::query::Query::from(query);
    move |event| {
        let Event {
            key,
            meta: Metadata {
                timestamp,
                tags,
                app_id,
            },
            payload,
        } = event;
        stream::iter(
            query
                .feed(Value::from((key, payload)))
                .into_iter()
                .map(move |v| EventResponse {
                    lamport: v.sort_key.lamport,
                    stream: v.sort_key.stream,
                    offset: v.sort_key.offset,
                    app_id: app_id.clone(),
                    timestamp,
                    tags: tags.clone(),
                    payload: v.payload(),
                }),
        )
        .boxed()
    }
}
