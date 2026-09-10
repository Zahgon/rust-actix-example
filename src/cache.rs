use crate::config::CONFIG;
use crate::errors::ApiError;
use axum::{Extension, Router};
use axum_extra::extract::cookie::Key;
use redis::{
    aio::{ConnectionManager, ConnectionManagerConfig},
    Value,
};
use std::time::Duration;

pub type Cache = ConnectionManager;

/// Retrieve an entry in redis
#[allow(dead_code)]
pub async fn get<'a>(redis: Cache, key: &'a str) -> Result<String, ApiError> {
    send(redis, &["GET", key]).await
}

/// Insert or update an entry in redis
#[allow(dead_code)]
pub async fn set<'a>(redis: Cache, key: &'a str, value: &'a str) -> Result<String, ApiError> {
    send(redis, &["SET", key, value]).await
}

/// Delete an entry in redis
#[allow(dead_code)]
pub async fn delete<'a>(redis: Cache, key: &'a str) -> Result<String, ApiError> {
    send(redis, &["DEL", key]).await
}

/// Send a command to redis over the shared connection
async fn send<'a>(mut redis: Cache, command: &[&'a str]) -> Result<String, ApiError> {
    let error_message = format!("Could not send {:?} command to Redis", command);
    let error = ApiError::CacheError(error_message);
    let mut cmd = redis::cmd(command[0]);
    for arg in &command[1..] {
        cmd.arg(arg);
    }
    let response: Value = cmd.query_async(&mut redis).await.map_err(|_| error)?;
    Ok(match response {
        Value::Okay => "OK".into(),
        Value::SimpleString(status) => status,
        Value::BulkString(bytes) => String::from_utf8(bytes).unwrap_or_else(|_| "".into()),
        _ => "".into(),
    })
}

/// Open a shared, auto-reconnecting connection to redis.
///
/// REDIS_URL is configured without a scheme (e.g. `127.0.0.1:6379`), so one is
/// added when missing.  Retries are capped so that an unreachable Redis does
/// not hold up application startup; the manager reconnects on its own once
/// Redis becomes available again.
pub async fn connect() -> Result<Cache, ApiError> {
    let url = if CONFIG.redis_url.contains("://") {
        CONFIG.redis_url.clone()
    } else {
        format!("redis://{}", CONFIG.redis_url)
    };
    let client =
        redis::Client::open(url).map_err(|error| ApiError::CacheError(error.to_string()))?;
    let config = ConnectionManagerConfig::new()
        .set_number_of_retries(0)
        .set_connection_timeout(Some(Duration::from_secs(1)));
    ConnectionManager::new_with_config(client, config)
        .await
        .map_err(|error| ApiError::CacheError(error.to_string()))
}

/// Add the redis connection to the router if the URL is set
pub async fn add_cache(router: Router<Key>) -> Router<Key> {
    if CONFIG.redis_url.is_empty() {
        return router;
    }
    match connect().await {
        Ok(cache) => router.layer(Extension(cache)),
        Err(error) => {
            tracing::warn!("Could not connect to Redis: {}", error);
            router
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_cache() -> Cache {
        connect().await.unwrap()
    }

    #[tokio::test]
    async fn it_creates_new_application_cache_and_sets_and_reads_it() {
        let cache = get_cache().await;
        set(cache.clone(), "testing", "123").await.unwrap();
        let value = get(cache, "testing").await.unwrap();
        assert_eq!(value, "123");
    }

    #[tokio::test]
    async fn it_removes_an_entry_in_application_cache() {
        let cache = get_cache().await;
        set(cache.clone(), "testing", "123").await.unwrap();
        let value = get(cache.clone(), "testing").await.unwrap();
        assert_eq!(value, "123");
        delete(cache.clone(), "testing").await.unwrap();
        let value = get(cache, "testing").await.unwrap();
        assert_eq!(value, "");
    }
}
