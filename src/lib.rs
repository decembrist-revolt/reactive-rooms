mod api;
mod auth;
mod domain;
mod message_bus;
mod storage;
mod websocket;

use api::{health, not_found};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::{Method, StatusCode, header},
    routing,
};
use axum_keycloak_auth::{
    NonEmpty, PassthroughMode,
    extract::{QueryParamTokenExtractor, TokenExtractor},
    layer::KeycloakAuthLayer,
};
use message_bus::MessageBus;
use mimalloc::MiMalloc;
use std::sync::Arc;
use std::time::Duration;
use storage::RoomStorage;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer, trace::TraceLayer};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

pub struct AppState {
    pub storage: RoomStorage,
    pub message_bus: MessageBus,
}

pub struct Server;

impl Server {
    async fn init_tcp_listener() -> TcpListener {
        let host = read_env_var("HOST", "0.0.0.0");
        let port = read_env_var("PORT", "3000");
        let addr = format!("{host}:{port}");

        TcpListener::bind(addr).await.expect("the address is busy")
    }

    fn init_router(state: Arc<AppState>) -> Router {
        let cors = Self::init_cors();
        let audience = auth::keycloak_audience();

        let ws_keycloak_layer = KeycloakAuthLayer::<auth::Role>::builder()
            .instance(auth::keycloak().clone())
            .passthrough_mode(PassthroughMode::Block)
            .persist_raw_claims(false)
            .expected_audiences(vec![audience])
            .token_extractors(NonEmpty::<Arc<dyn TokenExtractor>> {
                head: Arc::new(QueryParamTokenExtractor::default()),
                tail: vec![],
            })
            .build();

        let ws_routes = Router::new()
            .route("/websocket", routing::get(websocket::websocket_handler))
            .layer(ws_keycloak_layer);
        let rest_routes = api::routes::room_routes();
        let public_routes = Router::new().route("/api/health", routing::get(health));

        Router::new()
            .merge(public_routes)
            .merge(rest_routes)
            .merge(ws_routes)
            .fallback(not_found)
            .with_state(state)
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

        let origins = read_env_var("ORIGINS", "http://localhost:8080,http://127.0.0.1:8080")
            .trim_matches(|c| c == '[' || c == ']')
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| HeaderValue::from_str(s).expect("Invalid origin in ORIGINS"))
            .collect::<Vec<_>>();

        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST, Method::DELETE])
            .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
            .allow_origin(origins)
    }

    pub async fn run() {
        Self::init_tracing();

        auth::init_keycloak().expect("Failed to initialize Keycloak");
        let max_rooms = read_env_var("MAX_ROOMS", "10000").parse().unwrap_or(10_000);
        let state = Arc::new(AppState {
            storage: RoomStorage::new(max_rooms),
            message_bus: MessageBus::new(),
        });

        let listener = Self::init_tcp_listener().await;
        let router = Self::init_router(state);

        tracing::info!("listening on http://{}", listener.local_addr().unwrap());

        axum::serve(listener, router)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .unwrap()
    }
}

async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("received Ctrl+C, shutting down"),
        _ = terminate => tracing::info!("received SIGTERM, shutting down"),
    }
}

fn read_env_var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
