use crate::auth::{create_jwt, forget, hash, remember, PrivateClaim};
use crate::database::PoolType;
use crate::errors::ApiError;
use crate::handlers::user::UserResponse;
use crate::helpers::{respond_json, respond_ok};
use crate::models::user::find_by_auth;
use crate::validate::validate;
use crate::extractors::Json;
use axum::{response::Response, Extension};
use axum_extra::extract::cookie::PrivateCookieJar;
use serde::Serialize;
use tokio::task::spawn_blocking;
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Serialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "email must be a valid email"))]
    pub email: String,

    #[validate(length(
        min = 6,
        message = "password is required and must be at least 6 characters"
    ))]
    pub password: String,
}

/// Login a user
/// Create and remember their JWT
pub async fn login(
    jar: PrivateCookieJar,
    Extension(pool): Extension<PoolType>,
    Json(params): Json<LoginRequest>,
) -> Result<(PrivateCookieJar, Json<UserResponse>), ApiError> {
    validate(&params)?;

    // Validate that the email + hashed password matches
    let hashed = hash(&params.password);
    let user = spawn_blocking(move || find_by_auth(&pool, &params.email, &hashed)).await??;

    // Create a JWT
    let private_claim = PrivateClaim::new(user.id, user.email.clone());
    let jwt = create_jwt(private_claim)?;

    // Remember the token
    Ok((remember(jar, jwt), respond_json(user)?))
}

/// Logout a user
/// Forget their user_id
pub async fn logout(jar: PrivateCookieJar) -> Result<(PrivateCookieJar, Response), ApiError> {
    Ok((forget(jar), respond_ok()?))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::auth::get_identity_key;
    use crate::tests::helpers::tests::get_data_pool;
    use axum::http::HeaderMap;

    fn get_identity() -> PrivateCookieJar {
        PrivateCookieJar::from_headers(&HeaderMap::new(), get_identity_key())
    }

    async fn login_user() -> Result<(PrivateCookieJar, Json<UserResponse>), ApiError> {
        let params = LoginRequest {
            email: "satoshi@nakamotoinstitute.org".into(),
            password: "123456".into(),
        };
        let identity = get_identity();
        login(identity, get_data_pool(), Json(params)).await
    }

    async fn logout_user() -> Result<(PrivateCookieJar, Response), ApiError> {
        let identity = get_identity();
        logout(identity).await
    }

    #[tokio::test]
    async fn it_logs_a_user_in() {
        let response = login_user().await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn it_logs_a_user_out() {
        let _ = login_user().await.unwrap();
        let response = logout_user().await;
        assert!(response.is_ok());
    }
}
