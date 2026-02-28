use std::sync::Arc;

use axum::{Router, routing};
use axum_keycloak_auth::{PassthroughMode, layer::KeycloakAuthLayer};

use crate::{
    AppState,
    auth::{Role, keycloak, keycloak_audience},
};

use super::handlers;

pub fn room_routes() -> Router<Arc<AppState>> {
    let audience = keycloak_audience();
    let keycloak = keycloak();

    let keycloak_layer = KeycloakAuthLayer::<Role>::builder()
        .instance(keycloak.clone())
        .passthrough_mode(PassthroughMode::Block)
        .persist_raw_claims(false)
        .expected_audiences(vec![audience])
        .build();

    Router::new()
        .route(
            "/api/rooms",
            routing::post(handlers::create_room).get(handlers::list_rooms),
        )
        .route(
            "/api/rooms/{roomId}",
            routing::delete(handlers::cancel_room),
        )
        .layer(keycloak_layer)
}
