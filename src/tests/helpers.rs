#[cfg(test)]
pub mod tests {
    use crate::auth::get_identity_key;
    use crate::cache::add_cache;
    use crate::config::CONFIG;
    use crate::database::{add_pool, init_pool, Pool};
    use crate::handlers::auth::LoginRequest;
    use crate::routes::routes;
    use crate::state::{new_state, AppState};
    use axum::body::Body;
    use axum::http::{header, Request, Response};
    use axum::{Extension, Router};
    use diesel::mysql::MysqlConnection;
    use serde::Serialize;
    use tower::ServiceExt;

    /// Builds the application under test, wired up exactly like the server
    pub async fn app() -> Router {
        add_cache(add_pool(routes().layer(Extension(app_state()))))
            .await
            .with_state(get_identity_key())
    }

    /// Pull the `name=value` pair out of a response's Set-Cookie header
    fn session_cookie(response: &Response<Body>) -> String {
        response
            .headers()
            .get(header::SET_COOKIE)
            .expect("No session cookie was set")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned()
    }

    /// Helper for HTTP GET integration tests
    pub async fn test_get(route: &str) -> Response<Body> {
        let login_request = LoginRequest {
            email: "satoshi@nakamotoinstitute.org".into(),
            password: "123456".into(),
        };

        let app = app().await;

        let response = app
            .clone()
            .oneshot(
                Request::post("/api/v1/auth/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&login_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let cookie = session_cookie(&response);
        app.oneshot(
            Request::get(route)
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    /// Helper for HTTP POST integration tests
    pub async fn test_post<T: Serialize>(route: &str, params: T) -> Response<Body> {
        let app = app().await;
        let login = login().await;
        let cookie = session_cookie(&login);
        app.oneshot(
            Request::post(route)
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::COOKIE, cookie)
                .body(Body::from(serde_json::to_vec(&params).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    /// Assert that a route is successful for HTTP GET requests
    pub async fn assert_get(route: &str) -> Response<Body> {
        let response = test_get(route).await;
        assert!(response.status().is_success());
        response
    }

    /// Assert that a route is successful for HTTP POST requests
    pub async fn assert_post<T: Serialize>(route: &str, params: T) -> Response<Body> {
        let response = test_post(route, params).await;
        assert!(response.status().is_success());
        response
    }

    /// Returns a r2d2 Pooled Connection to be used in tests
    pub fn get_pool() -> Pool<MysqlConnection> {
        init_pool::<MysqlConnection>(CONFIG.clone()).unwrap()
    }

    /// Returns a r2d2 Pooled Connection wrapped in an Axum Extension
    pub fn get_data_pool() -> Extension<Pool<MysqlConnection>> {
        Extension(get_pool())
    }

    /// Login to routes
    pub async fn login() -> Response<Body> {
        let login_request = LoginRequest {
            email: "satoshi@nakamotoinstitute.org".into(),
            password: "123456".into(),
        };
        let app = add_pool(routes())
            .with_state(get_identity_key());
        app.oneshot(
            Request::post("/api/v1/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&login_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap()
    }

    // Mock applicate state
    pub fn app_state() -> AppState<'static, String> {
        new_state::<String>()
    }
}
