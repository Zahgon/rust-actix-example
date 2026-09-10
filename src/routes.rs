//! Place all Axum routes here, multiple route configs can be used and
//! combined.

use crate::auth::get_identity_key;
use crate::handlers::{
    auth::{login, logout},
    health::get_health,
    user::{create_user, delete_user, get_user, get_users, update_user},
};
use crate::middleware::auth::auth as auth_middleware;
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::Key;
use tower_http::services::ServeDir;

pub fn routes() -> Router<Key> {
    let key = get_identity_key();

    // /api/v1 routes, locked down with the AUTH middleware
    let api = Router::<Key>::new()
        // AUTH routes
        .route("/api/v1/auth/login", post(login))
        .route("/api/v1/auth/logout", get(logout))
        // USER routes
        .route(
            "/api/v1/user/{id}",
            get(get_user).put(update_user).delete(delete_user),
        )
        .route("/api/v1/user", get(get_users).post(create_user))
        .layer(middleware::from_fn_with_state(key.clone(), auth_middleware));

    // Serve secure static files from the static-secure folder,
    // also locked down with the AUTH middleware
    let secure = Router::<Key>::new()
        .nest_service(
            "/secure",
            ServeDir::new("./static-secure").append_index_html_on_directories(true),
        )
        .layer(middleware::from_fn_with_state(key, auth_middleware));

    Router::<Key>::new()
        // Healthcheck
        .route("/health", get(get_health))
        .merge(api)
        .merge(secure)
        // Serve public static files from the static folder
        .fallback_service(ServeDir::new("./static").append_index_html_on_directories(true))
}
