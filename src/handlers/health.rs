use crate::errors::ApiError;
use crate::helpers::respond_json;
use crate::extractors::Json;

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

/// Handler to get the liveness of the service
pub async fn get_health() -> Result<Json<HealthResponse>, ApiError> {
    respond_json(HealthResponse {
        status: "ok".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_health() {
        let response = get_health().await.unwrap();
        assert_eq!(response.0.status, "ok".to_string());
    }
}
