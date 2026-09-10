use crate::config::CONFIG;
use crate::errors::ApiError;
use argon2rs::argon2i_simple;
use axum_extra::extract::cookie::{Cookie, Key, PrivateCookieJar};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PrivateClaim {
    pub user_id: Uuid,
    pub email: String,
    exp: i64,
}

impl PrivateClaim {
    pub fn new(user_id: Uuid, email: String) -> Self {
        Self {
            user_id,
            email,
            exp: (Utc::now() + Duration::hours(CONFIG.jwt_expiration)).timestamp(),
        }
    }
}

/// Create a json web token (JWT)
pub fn create_jwt(private_claim: PrivateClaim) -> Result<String, ApiError> {
    let encoding_key = EncodingKey::from_secret(&CONFIG.jwt_key.as_ref());
    encode(
        &Header::default(),
        &private_claim,
        &encoding_key,
    )
    .map_err(|e| ApiError::CannotEncodeJwtToken(e.to_string()))
}

/// Decode a json web token (JWT)
pub fn decode_jwt(token: &str) -> Result<PrivateClaim, ApiError> {
    let decoding_key = DecodingKey::from_secret(&CONFIG.jwt_key.as_ref());
    decode::<PrivateClaim>(token, &decoding_key, &Validation::default())
        .map(|data| data.claims)
        .map_err(|e| ApiError::CannotDecodeJwtToken(e.to_string()))
}

/// Encrypt a password
///
/// Uses the argon2i algorithm.
/// auth_salt is environment-configured.
pub fn hash(password: &str) -> String {
    argon2i_simple(&password, &CONFIG.auth_salt)
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Gets the key used to encrypt/decrypt the private identity cookie.
///
/// This is the Axum application state, which makes `PrivateCookieJar`
/// extractable from any handler.
pub fn get_identity_key() -> Key {
    Key::derive_from(&CONFIG.session_key.as_ref())
}

/// Remember an identity by storing the JWT in a private (encrypted) cookie.
///
/// The returned jar must be returned from the handler for the cookie to be set.
pub fn remember(jar: PrivateCookieJar, jwt: String) -> PrivateCookieJar {
    let cookie = Cookie::build((CONFIG.session_name.clone(), jwt))
        .path("/")
        .secure(CONFIG.session_secure)
        .http_only(true)
        .max_age(time::Duration::minutes(CONFIG.session_timeout))
        .build();
    jar.add(cookie)
}

/// Forget an identity by removing the private identity cookie.
pub fn forget(jar: PrivateCookieJar) -> PrivateCookieJar {
    let cookie = Cookie::build(CONFIG.session_name.clone())
        .path("/")
        .http_only(true)
        .build();
    jar.remove(cookie)
}

/// Pull the JWT out of the private identity cookie, if one was sent.
pub fn get_identity(jar: &PrivateCookieJar) -> Option<String> {
    jar.get(&CONFIG.session_name)
        .map(|cookie| cookie.value().to_string())
}

#[cfg(test)]
pub mod tests {
    use super::*;
    static EMAIL: &str = "test@test.com";

    #[test]
    fn it_hashes_a_password() {
        let password = "password";
        let hashed = hash(password);
        assert_ne!(password, hashed);
    }

    #[test]
    fn it_matches_2_hashed_passwords() {
        let password = "password";
        let hashed = hash(password);
        let hashed_again = hash(password);
        assert_eq!(hashed, hashed_again);
    }

    #[test]
    fn it_creates_a_jwt() {
        let private_claim = PrivateClaim::new(Uuid::new_v4(), EMAIL.into());
        let jwt = create_jwt(private_claim);
        assert!(jwt.is_ok());
    }

    #[test]
    fn it_decodes_a_jwt() {
        let private_claim = PrivateClaim::new(Uuid::new_v4(), EMAIL.into());
        let jwt = create_jwt(private_claim.clone()).unwrap();
        let decoded = decode_jwt(&jwt).unwrap();
        assert_eq!(private_claim, decoded);
    }
}
