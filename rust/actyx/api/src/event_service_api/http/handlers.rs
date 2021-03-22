use super::ndjson;

use actyxos_sdk::{
    service::{EventService, PublishRequest, QueryRequest, SubscribeMonotonicRequest, SubscribeRequest},
    AppId,
};

use derive_more::Display;
use warp::*;

#[derive(Debug, Display)]
pub struct Error(anyhow::Error); // anyhow::Error is sealed so we wrap it
impl std::error::Error for Error {}
impl reject::Reject for Error {}

fn reject(err: anyhow::Error) -> Rejection {
    reject::custom(Error(err))
}

type Result<T> = std::result::Result<T, Rejection>;

pub async fn node_id(_app_id: AppId, event_service: impl EventService) -> Result<impl Reply> {
    event_service
        .node_id()
        .await
        .map(|reply| reply::json(&reply))
        .map(|reply| reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache"))
        .map_err(reject)
}

pub async fn offsets(_app_id: AppId, event_service: impl EventService) -> Result<impl Reply> {
    event_service
        .offsets()
        .await
        .map(|reply| reply::json(&reply))
        .map(|reply| reply::with_header(reply, http::header::CACHE_CONTROL, "no-cache"))
        .map_err(reject)
}

pub async fn publish(_app_id: AppId, request: PublishRequest, event_service: impl EventService) -> Result<impl Reply> {
    event_service
        .publish(request)
        .await
        .map(|reply| reply::json(&reply))
        .map_err(reject)
}

pub async fn query(_app_id: AppId, request: QueryRequest, event_service: impl EventService) -> Result<impl Reply> {
    event_service
        .query(request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

pub async fn subscribe(
    _app_id: AppId,
    request: SubscribeRequest,
    event_service: impl EventService,
) -> Result<impl Reply> {
    event_service
        .subscribe(request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}

pub async fn subscribe_monotonic(
    _app_id: AppId,
    request: SubscribeMonotonicRequest,
    event_service: impl EventService,
) -> Result<impl Reply> {
    event_service
        .subscribe_monotonic(request)
        .await
        .map(|events| ndjson::reply(ndjson::keep_alive().stream(events)))
        .map_err(reject)
}