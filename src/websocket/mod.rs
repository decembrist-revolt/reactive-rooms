mod host;
mod ping;
mod user;

use crate::{
    AppState,
    auth::{Role, has_role},
    domain::{room::RoomId, user::UserId},
};
use axum::{
    Extension,
    extract::{Query, State, WebSocketUpgrade},
    http::StatusCode,
    response::IntoResponse,
};
use axum_keycloak_auth::decode::KeycloakToken;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct WsQueryParams {
    #[serde(rename = "roomId")]
    room_id: String,
    #[serde(rename = "type")]
    connection_type: String,
}

pub async fn websocket_handler(
    Extension(token): Extension<KeycloakToken<Role>>,
    Query(params): Query<WsQueryParams>,
    State(state): State<Arc<AppState>>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    let user_id = UserId::new(&token.subject);
    let room_id = match params.room_id.parse::<RoomId>() {
        Ok(id) => id,
        Err(_) => {
            tracing::warn!(
                "WebSocket connection with invalid room ID {}",
                params.room_id
            );
            return (StatusCode::BAD_REQUEST, "Invalid room ID").into_response();
        }
    };

    let room = match state.storage.get_room(&room_id) {
        Some(room) => room,
        None => {
            tracing::warn!("WebSocket connection to non-existent room {}", room_id);
            return (StatusCode::NOT_FOUND, "Room not found").into_response();
        }
    };

    match params.connection_type.as_str() {
        "host" => {
            if !has_role(&token, &Role::Host) {
                tracing::warn!(
                    "User {} attempted host connection without host role",
                    token.subject
                );
                return (StatusCode::FORBIDDEN, "Host role required").into_response();
            }

            if !room.is_host(&user_id) {
                tracing::warn!(
                    "User {} attempted host connection to room {} but is not the host",
                    token.subject,
                    room_id
                );
                return (StatusCode::FORBIDDEN, "Not the room host").into_response();
            }

            tracing::info!("Host {} connecting to room {}", token.subject, room_id);

            ws.on_upgrade(move |socket| host::handle_host_ws(socket, state, room_id, user_id))
                .into_response()
        }
        "user" => {
            if !has_role(&token, &Role::User) {
                tracing::warn!(
                    "User {} attempted connection without user role",
                    token.subject
                );
                return (StatusCode::FORBIDDEN, "User role required").into_response();
            }

            tracing::info!("User {} connecting to room {}", token.subject, room_id);

            ws.on_upgrade(move |socket| user::handle_user_ws(socket, state, room_id, user_id))
                .into_response()
        }
        _ => (StatusCode::BAD_REQUEST, "Invalid connection type").into_response(),
    }
}
