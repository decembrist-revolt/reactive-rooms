use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_keycloak_auth::{decode::KeycloakToken, expect_role};

use crate::{
    AppState,
    auth::Role,
    api::dto::{
        CreateRoomRequest, CreateRoomResponse, PaginationParams,
        RoomWithPlayerCount, RoomsPageResponse,
    },
    domain::{
        event::DisconnectReason,
        room::{Room, RoomType},
        user::UserId,
    },
};

pub async fn create_room(
    Extension(token): Extension<KeycloakToken<Role>>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    expect_role!(&token, Role::Admin);

    let room = Room::new(
        UserId::new(&body.host_id),
        RoomType::new(&body.room_type),
    );

    match state.storage.create_room(room) {
        Ok(room_id) => {
            tracing::info!(
                "Room {} created by user {} for host {} and type {}",
                room_id, token.subject, body.host_id, body.room_type,
            );
            (
                StatusCode::CREATED,
                Json(CreateRoomResponse {
                    room_id: room_id.to_string(),
                }),
            )
                .into_response()
        }
        Err(_) => {
            tracing::error!(
                "Failed to create room for host {} and type {}: room already exists",
                body.host_id, body.room_type,
            );
            (StatusCode::CONFLICT, "Room already exists").into_response()
        }
    }
}

pub async fn cancel_room(
    Extension(token): Extension<KeycloakToken<Role>>,
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
) -> impl IntoResponse {
    expect_role!(&token, Role::Admin);

    let room = match state.storage.get_room(&room_id) {
        Some(room) => room,
        None => {
            tracing::warn!("Attempted to delete non-existent room {}", room_id);
            return (StatusCode::NOT_FOUND, "Room not found").into_response();
        }
    };

    // Get all users before removing the room
    let users = state.storage.clear_room_users(&room_id);

    // Disconnect all users
    state.message_bus.disconnect_room_users(
        &room_id,
        &users,
        DisconnectReason::RoomClosed,
    );

    // Disconnect host
    state.message_bus.disconnect_host(
        &room_id,
        &room.host_id,
        DisconnectReason::RoomClosed,
    );

    // Remove room
    state.storage.remove_room(&room_id);

    tracing::info!("Room {} deleted by user {}", room_id, token.subject);
    StatusCode::NO_CONTENT.into_response()
}

pub async fn list_rooms(
    Extension(token): Extension<KeycloakToken<Role>>,
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> impl IntoResponse {
    expect_role!(&token, Role::Admin);

    let page = params.page.unwrap_or(0);
    let size = params.size.unwrap_or(10);

    if size == 0 || size > 100 {
        return (StatusCode::BAD_REQUEST, "Invalid pagination parameters").into_response();
    }

    let (rooms, total) = state.storage.get_rooms_paginated(page, size);
    let rooms: Vec<RoomWithPlayerCount> = rooms
        .into_iter()
        .map(|room| {
            let room_id_str = room.id.to_string();
            let player_count = state.storage.get_room_user_count(&room_id_str);
            RoomWithPlayerCount {
                room_id: room_id_str,
                host_id: room.host_id.as_str().to_string(),
                room_type: room.room_type.as_str().to_string(),
                player_count,
            }
        })
        .collect();

    tracing::info!(
        "Retrieved rooms page {} with size {}, total rooms: {}",
        page, size, total,
    );

    Json(RoomsPageResponse {
        rooms,
        total_rooms: total,
        page,
        size,
    })
    .into_response()
}
