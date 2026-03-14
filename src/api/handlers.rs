use crate::{
    AppState,
    api::{
        dto::{
            CreateRoomRequest, CreateRoomResponse, PaginationParams, RoomWithPlayerCount,
            RoomsPageResponse,
        },
        error_response,
    },
    auth::Role,
    domain::{
        event::DisconnectReason,
        room::{Room, RoomId, RoomType},
        user::UserId,
    },
    storage::CreateRoomError,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_keycloak_auth::{decode::KeycloakToken, expect_role};
use std::sync::Arc;

pub async fn create_room(
    Extension(token): Extension<KeycloakToken<Role>>,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateRoomRequest>,
) -> impl IntoResponse {
    expect_role!(&token, Role::Admin);

    if body.host_id.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "hostId must not be empty").into_response();
    }

    if body.room_type.is_empty() {
        return error_response(StatusCode::BAD_REQUEST, "type must not be empty").into_response();
    }

    let room = Room::new(UserId::new(&body.host_id), RoomType::new(&body.room_type));

    match state.storage.create_room(room) {
        Ok(room_id) => {
            tracing::info!(
                "Room {} created by user {} for host {} and type {}",
                room_id,
                token.subject,
                body.host_id,
                body.room_type,
            );
            (
                StatusCode::CREATED,
                Json(CreateRoomResponse {
                    room_id: room_id.to_string(),
                }),
            )
                .into_response()
        }
        Err(CreateRoomError::RoomAlreadyExists) => {
            tracing::error!(
                "Failed to create room for host {} and type {}: room already exists",
                body.host_id,
                body.room_type,
            );
            error_response(StatusCode::CONFLICT, "Room already exists").into_response()
        }
        Err(CreateRoomError::RoomLimitReached) => {
            tracing::error!("Room limit reached");
            error_response(StatusCode::SERVICE_UNAVAILABLE, "Room limit reached").into_response()
        }
    }
}

pub async fn cancel_room(
    Extension(token): Extension<KeycloakToken<Role>>,
    State(state): State<Arc<AppState>>,
    Path(room_id_str): Path<String>,
) -> impl IntoResponse {
    expect_role!(&token, Role::Admin);

    let room_id = match room_id_str.parse::<RoomId>() {
        Ok(id) => id,
        Err(_) => {
            return error_response(StatusCode::BAD_REQUEST, "Invalid room ID").into_response();
        }
    };

    let (room, users) = match state.storage.remove_room_with_users(&room_id) {
        Some(result) => result,
        None => {
            tracing::warn!("Attempted to delete non-existent room {}", room_id);
            return error_response(StatusCode::NOT_FOUND, "Room not found").into_response();
        }
    };

    state
        .message_bus
        .disconnect_room_users(&room_id, &users, DisconnectReason::RoomClosed);

    state
        .message_bus
        .disconnect_host(&room_id, &room.host_id, DisconnectReason::RoomClosed);

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
        return error_response(StatusCode::BAD_REQUEST, "Invalid pagination parameters")
            .into_response();
    }

    let (rooms, total) = state.storage.get_rooms_paginated(page, size);
    let rooms: Vec<RoomWithPlayerCount> = rooms
        .into_iter()
        .map(|room| {
            let player_count = state.storage.get_room_user_count(&room.id);
            RoomWithPlayerCount {
                room_id: room.id.to_string(),
                host_id: room.host_id.as_str().to_string(),
                room_type: room.room_type.as_str().to_string(),
                player_count,
            }
        })
        .collect();

    tracing::info!(
        "Retrieved rooms page {} with size {}, total rooms: {}",
        page,
        size,
        total,
    );

    Json(RoomsPageResponse {
        rooms,
        total_rooms: total,
        page,
        size,
    })
    .into_response()
}
