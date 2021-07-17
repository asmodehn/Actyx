use actyx_sdk::AppId;
use actyx_util::serde_support::StringSerialized;
use derive_more::Display;
use tracing::*;
use warp::{http::StatusCode, *};

#[derive(Debug, Display, Clone)]
pub enum ApiError {
    #[display(fmt = "The requested resource could not be found.")]
    NotFound,

    #[display(fmt = "Method not supported.")]
    MethodNotAllowed,

    #[display(
        fmt = "Content with type '{}' was requested but the resource is only capable of generating content of the following type(s): {}.",
        requested,
        supported
    )]
    NotAcceptable { supported: String, requested: String },

    #[display(fmt = "Invalid manifest. {}", msg)]
    InvalidManifest { msg: String },

    #[display(
        fmt = "'{}' is not authorized. {}. Provide a valid app license to the node.",
        app_id,
        reason
    )]
    AppUnauthorized { app_id: AppId, reason: String },

    #[display(fmt = "\"Authorization\" header is missing.")]
    MissingAuthorizationHeader,

    #[display(fmt = "\"token\" parameter is missing.")]
    MissingTokenParameter,

    #[display(fmt = "Unauthorized token.")]
    TokenUnauthorized,

    #[display(fmt = "Expired token.")]
    TokenExpired,

    #[display(fmt = "Invalid token: '{}'. {} Please provide a valid bearer token.", token, msg)]
    TokenInvalid { token: String, msg: String },

    #[display(fmt = "{}.", msg)]
    UnsupportedMediaType { msg: String },

    #[display(
        fmt = "Unsupported authentication type '{}'. Only \"Bearer\" is supported.",
        requested
    )]
    UnsupportedAuthType { requested: String },

    #[display(fmt = "Invalid request. {}", cause)]
    BadRequest { cause: String },

    #[display(fmt = "Internal server error.")]
    Internal,

    #[display(fmt = "Service overloaded. {}", cause)]
    Overloaded { cause: String },

    #[display(fmt = "Service shutting down. {}", cause)]
    Shutdown { cause: String },
}
impl warp::reject::Reject for ApiError {}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
    #[serde(skip)]
    pub status: StatusCode,
    pub code: String,
    pub message: StringSerialized<ApiError>,
}
impl From<ApiError> for ApiErrorResponse {
    fn from(e: ApiError) -> Self {
        let (status, code) = match &e {
            ApiError::AppUnauthorized { .. } => (StatusCode::UNAUTHORIZED, "ERR_APP_UNAUTHORIZED"),
            ApiError::BadRequest { .. } => (StatusCode::BAD_REQUEST, "ERR_BAD_REQUEST"),
            ApiError::Internal => (StatusCode::INTERNAL_SERVER_ERROR, "ERR_INTERNAL"),
            ApiError::InvalidManifest { .. } => (StatusCode::BAD_REQUEST, "ERR_MANIFEST_INVALID"),
            ApiError::MethodNotAllowed => (StatusCode::METHOD_NOT_ALLOWED, "ERR_METHOD_NOT_ALLOWED"),
            ApiError::MissingAuthorizationHeader => (StatusCode::UNAUTHORIZED, "ERR_MISSING_AUTH_HEADER"),
            ApiError::MissingTokenParameter => (StatusCode::UNAUTHORIZED, "ERR_MISSING_TOKEN_PARAM"),
            ApiError::NotAcceptable { .. } => (StatusCode::NOT_ACCEPTABLE, "ERR_NOT_ACCEPTABLE"),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "ERR_NOT_FOUND"),
            ApiError::TokenExpired => (StatusCode::UNAUTHORIZED, "ERR_TOKEN_EXPIRED"),
            ApiError::TokenInvalid { .. } => (StatusCode::BAD_REQUEST, "ERR_TOKEN_INVALID"),
            ApiError::TokenUnauthorized => (StatusCode::UNAUTHORIZED, "ERR_TOKEN_UNAUTHORIZED"),
            ApiError::UnsupportedAuthType { .. } => (StatusCode::UNAUTHORIZED, "ERR_UNSUPPORTED_AUTH_TYPE"),
            ApiError::UnsupportedMediaType { .. } => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "ERR_UNSUPPORTED_MEDIA_TYPE"),
            ApiError::Overloaded { .. } => (StatusCode::SERVICE_UNAVAILABLE, "ERR_SERVICE_OVERLOADED"),
            ApiError::Shutdown { .. } => (StatusCode::SERVICE_UNAVAILABLE, "ERR_SHUTTING_DOWN"),
        };
        ApiErrorResponse {
            code: code.to_string(),
            status,
            message: e.into(),
        }
    }
}

/// Internal rejection used for testing purposes
#[derive(Debug)]
pub(crate) struct Crash;
impl reject::Reject for Crash {}

pub fn handle_rejection(r: Rejection) -> Result<impl Reply, Rejection> {
    let api_err = if r.is_not_found() {
        ApiError::NotFound
    } else if r.find::<reject::MethodNotAllowed>().is_some() {
        ApiError::MethodNotAllowed
    } else if let Some(umt) = r.find::<reject::UnsupportedMediaType>() {
        ApiError::UnsupportedMediaType { msg: umt.to_string() }
    } else if let Some(e) = r.find::<ApiError>() {
        if let ApiError::AppUnauthorized { app_id, reason } = e {
            info!(target: "AUTH", "Unauthorized app {}. {}.", app_id, reason)
        }
        e.to_owned()
    } else if let Some(e) = r.find::<filters::body::BodyDeserializeError>() {
        use std::error::Error;
        ApiError::BadRequest {
            cause: e.source().map_or("unknown".to_string(), |e| e.to_string()),
        }
    } else {
        warn!("unhandled rejection: {:?}", r);
        ApiError::Internal
    };

    let err_resp: ApiErrorResponse = api_err.into();
    let json = warp::reply::json(&err_resp);
    Ok(warp::reply::with_status(json, err_resp.status))
}
