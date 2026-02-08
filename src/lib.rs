mod api;
mod auth;
mod domain;
mod message_bus;
mod storage;
mod websocket;

use api::{not_found, ping};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{Method, StatusCode, header},
    routing,
};
use mimalloc::MiMalloc;
use std::time::Duration;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub struct Server;

impl Server {
    async fn init_tcp_listener() -> TcpListener {
        let host = read_env_var("HOST", "0.0.0.0");
        let port = read_env_var("PORT", "3000");
        let addr = format!("{host}:{port}");

        TcpListener::bind(addr).await.expect("the address is busy")
    }

    fn init_router() -> Router {
        let cors = Self::init_cors();
        Router::new()
            .route("/ping", routing::get(ping))
            .fallback(not_found)
            .layer(cors)
            .layer((
                TraceLayer::new_for_http(),
                TimeoutLayer::with_status_code(
                    StatusCode::REQUEST_TIMEOUT,
                    Duration::from_secs(10),
                ),
                DefaultBodyLimit::max(2 * 1024 * 1024),
            ))
    }

    fn init_tracing() {
        use tracing_subscriber::EnvFilter;

        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .compact()
            .with_file(true)
            .with_line_number(true)
            .with_target(false)
            .init();
    }

    fn init_cors() -> CorsLayer {
        use axum::http::HeaderValue;

        let origins = read_env_var("ORIGINS", "[http://localhost:8080,http://127.0.0.1:8080]")
            .split(',')
            .map(|s| s.trim())
            .map(|s| HeaderValue::from_str(s).expect("Invalid origin in ORIGINS"))
            .collect::<Vec<_>>();

        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
            .allow_origin(origins)
    }

    pub async fn run() {
        Self::init_tracing();
        let listener = Self::init_tcp_listener().await;
        let router = Self::init_router();

        tracing::info!("listening on http://{}", listener.local_addr().unwrap());

        axum::serve(listener, router).await.unwrap()
    }
}

fn read_env_var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
