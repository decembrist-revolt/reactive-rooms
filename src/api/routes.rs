use std::sync::Arc;

use axum::{Router, routing};
use axum_keycloak_auth::{
    layer::KeycloakAuthLayer,
    PassthroughMode,
};

use crate::{AppState, auth::Role};

use super::handlers;

pub fn room_routes(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let audience =
        std::env::var("KEYCLOAK_AUDIENCE").unwrap_or_else(|_| "account".to_string());

    let keycloak_layer = KeycloakAuthLayer::<Role>::builder()
        .instance(state.keycloak.clone())
        .passthrough_mode(PassthroughMode::Block)
        .persist_raw_claims(false)
        .expected_audiences(vec![audience])
        .build();

    Router::new()
        .route(
            "/api/rooms",
            routing::post(handlers::create_room).get(handlers::list_rooms),
        )
        .route("/api/rooms/{roomId}", routing::delete(handlers::cancel_room))
        .layer(keycloak_layer)
}
