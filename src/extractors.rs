use crate::auth::{decode_jwt, get_identity, PrivateClaim};
use crate::models::user::AuthUser;
use axum::{
    extract::{
        path::ErrorKind, rejection::PathRejection, FromRef, FromRequest, FromRequestParts, Request,
    },
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
};
use axum_extra::extract::cookie::{Key, PrivateCookieJar};
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Extractor for pulling the identity out of a request.
///
/// Simply add "user: AuthUser" to a handler to invoke this.
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
    Key: FromRef<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let jar = PrivateCookieJar::<Key>::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;
        let identity = get_identity(&jar);
        if let Some(identity) = identity {
            let private_claim: PrivateClaim = decode_jwt(&identity).unwrap();
            return Ok(AuthUser {
                id: private_claim.user_id.to_string(),
                email: private_claim.email,
            });
        }
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Path extractor.
///
/// Wraps `axum::extract::Path` so that a path segment which cannot be
/// deserialized is reported as `404 Not Found` rather than Axum's default
/// `400 Bad Request`, and so that the body is the bare deserializer message
/// rather than Axum's "Invalid URL: Cannot parse ..." wrapper.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Path<T>(pub T);

impl<S, T> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Path::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Path(value)) => Ok(Path(value)),
            Err(rejection) => {
                let message = match &rejection {
                    PathRejection::FailedToDeserializePathParams(error) => match error.kind() {
                        ErrorKind::DeserializeError { message, .. } => message.clone(),
                        ErrorKind::Message(message) => message.clone(),
                        kind => kind.to_string(),
                    },
                    other => other.body_text(),
                };
                Err((StatusCode::NOT_FOUND, message).into_response())
            }
        }
    }
}

/// Json extractor and responder.
///
/// Wraps `axum::Json` so that every request payload problem - malformed body,
/// missing fields, or a missing `Content-Type` header - is reported as an
/// empty `400 Bad Request`, instead of Axum's mix of 400/415/422 with a
/// descriptive body.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Json<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(value)) => Ok(Json(value)),
            Err(_) => Err(StatusCode::BAD_REQUEST.into_response()),
        }
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}
