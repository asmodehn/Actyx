use actyx_sdk::AppId;
use warp::filters::*;
use warp::*;

use crate::events::service::EventService;
use crate::{
    events::http::handlers,
    util::filters::{accept_json, accept_ndjson},
};

fn with_service(
    event_service: EventService,
) -> impl Filter<Extract = (EventService,), Error = std::convert::Infallible> + Clone {
    any().map(move || event_service.clone())
}

pub fn offsets(
    event_service: EventService,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("offsets"))
        .and(get())
        .and(accept_json())
        .and(with_service(event_service))
        .and_then(handlers::offsets)
}

pub fn publish(
    event_service: EventService,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("publish"))
        .and(post())
        .and(accept_json())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::publish)
}

pub fn query(
    event_service: EventService,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("query"))
        .and(post())
        .and(accept_ndjson())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::query)
}

pub fn subscribe(
    event_service: EventService,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("subscribe"))
        .and(post())
        .and(accept_ndjson())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::subscribe)
}

pub fn subscribe_monotonic(
    event_service: EventService,
    auth: impl Filter<Extract = (AppId,), Error = Rejection> + Clone,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    auth.and(path!("subscribe_monotonic"))
        .and(post())
        .and(accept_ndjson())
        .and(body::json())
        .and(with_service(event_service))
        .and_then(handlers::subscribe_monotonic)
}
