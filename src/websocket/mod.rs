mod host;
mod user;

use std::sync::Arc;

use axum::{
    Extension,
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use axum_keycloak_auth::decode::KeycloakToken;

use crate::{
    AppState,
    api::dto::WsQueryParams,
    auth::{Role, has_role},
    domain::user::UserId,
};

pub async fn websocket_handler(
    Extension(token): Extension<KeycloakToken<Role>>,
    Query(params): Query<WsQueryParams>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user_id = UserId::new(&token.subject);
    let room_id_str = params.room_id.clone();

    // Validate room exists
    let room = match state.storage.get_room(&room_id_str) {
        Some(room) => room,
        None => {
            tracing::warn!("WebSocket connection to non-existent room {}", room_id_str);
            return (StatusCode::NOT_FOUND, "Room not found").into_response();
        }
    };

    match params.connection_type.as_str() {
        "host" => {
            // Verify user has host role
            if !has_role(&token, &Role::Host) {
                tracing::warn!(
                    "User {} attempted host connection without host role",
                    token.subject
                );
                return (StatusCode::FORBIDDEN, "Host role required").into_response();
            }

            // Verify user is the room's host
            if !room.is_host(&user_id) {
                tracing::warn!(
                    "User {} attempted host connection to room {} but is not the host",
                    token.subject, room_id_str
                );
                return (StatusCode::FORBIDDEN, "Not the room host").into_response();
            }

            tracing::info!(
                "Host {} connecting to room {}",
                token.subject, room_id_str
            );

            ws.on_upgrade(move |socket| {
                host::handle_host_ws(socket, state, room_id_str, user_id)
            })
            .into_response()
        }
        "user" => {
            // Verify user has user role
            if !has_role(&token, &Role::User) {
                tracing::warn!(
                    "User {} attempted connection without user role",
                    token.subject
                );
                return (StatusCode::FORBIDDEN, "User role required").into_response();
            }

            tracing::info!(
                "User {} connecting to room {}",
                token.subject, room_id_str
            );

            ws.on_upgrade(move |socket| {
                user::handle_user_ws(socket, state, room_id_str, user_id)
            })
            .into_response()
        }
        _ => (StatusCode::BAD_REQUEST, "Invalid connection type").into_response(),
    }
}
