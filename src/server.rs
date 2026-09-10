//! Spin up an Axum HTTP server

use crate::auth::get_identity_key;
use crate::cache::add_cache;
use crate::config::CONFIG;
use crate::database::add_pool;
use crate::routes::routes;
use crate::state::new_state;
use axum::Extension;
use listenfd::ListenFd;
use tokio::net::TcpListener;
use axum::http::Method;
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

pub async fn server() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Create the application state
    // String is used here, but it can be anything
    // Invoke in handlers using Extension(data): Extension<AppState<'_, String>>
    let data = new_state::<String>();

    // Mirror the request for origins/methods/headers so that credentialed
    // cross-origin requests are supported
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_methods([
            Method::GET,
            Method::HEAD,
            Method::POST,
            Method::OPTIONS,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true);

    let app = add_cache(add_pool(routes().layer(Extension(data))))
        .await
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(get_identity_key());

    let mut listenfd = ListenFd::from_env();
    let listener = if let Some(listener) = listenfd.take_tcp_listener(0)? {
        listener.set_nonblocking(true)?;
        TcpListener::from_std(listener)?
    } else {
        TcpListener::bind(&CONFIG.server).await?
    };

    axum::serve(listener, app).await
}
