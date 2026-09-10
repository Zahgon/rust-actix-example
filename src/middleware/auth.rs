use crate::auth::{decode_jwt, get_identity, PrivateClaim};
use crate::errors::ApiError;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Key, PrivateCookieJar};

/// Reject requests that do not carry a valid identity JWT.
///
/// The login route is always allowed through so that a session can be created.
pub async fn auth(State(key): State<Key>, req: Request, next: Next) -> Response {
    let jar = PrivateCookieJar::from_headers(req.headers(), key);
    let identity = get_identity(&jar).unwrap_or_else(|| "".into());
    let private_claim: Result<PrivateClaim, ApiError> = decode_jwt(&identity);
    let is_logged_in = private_claim.is_ok();
    let unauthorized = !is_logged_in && req.uri().path() != "/api/v1/auth/login";

    if unauthorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    next.run(req).await
}
