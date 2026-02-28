use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::time::{Instant, interval};

use crate::{
    AppState,
    domain::{
        event::{DisconnectReason, ToHostEvent},
        message::{HostWebSocketMessage, ToUserMessage},
        user::UserId,
    },
};

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn handle_host_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    room_id: String,
    host_id: UserId,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut bus_rx = state.message_bus.register_host(&room_id);
    let mut ping_interval = interval(PING_INTERVAL);
    ping_interval.tick().await; // consume first immediate tick
    let mut pong_deadline: Option<Instant> = None;

    loop {
        tokio::select! {
            // Message from a user via the bus -> forward to host WS
            msg = bus_rx.recv() => {
                match msg {
                    Some(msg) => {
                        // If this is a disconnect message for the host, break
                        if matches!(msg.event, ToHostEvent::Disconnect)
                            && msg.user_id == host_id
                        {
                            let json = serde_json::to_string(&msg).unwrap();
                            let _ = ws_sender.send(WsMessage::Text(json.into())).await;
                            break;
                        }

                        match serde_json::to_string(&msg) {
                            Ok(json) => {
                                if ws_sender.send(WsMessage::Text(json.into())).await.is_err() {
                                    tracing::error!("Failed to send message to host {}", host_id.as_str());
                                    break;
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to serialize message for host: {}", e);
                            }
                        }
                    }
                    None => {
                        // Channel closed
                        break;
                    }
                }
            }

            // Message from host WS -> route to target user
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        handle_host_message(&state, &room_id, &host_id, &text);
                    }
                    Some(Ok(WsMessage::Pong(_))) => {
                        pong_deadline = None;
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error for host {}: {}", host_id.as_str(), e);
                        break;
                    }
                    _ => {}
                }
            }

            // Ping tick
            _ = ping_interval.tick() => {
                if let Some(deadline) = pong_deadline
                    && Instant::now() > deadline {
                        tracing::warn!("Host {} pong timeout, disconnecting", host_id.as_str());
                        break;
                    }
                if ws_sender.send(WsMessage::Ping(vec![].into())).await.is_err() {
                    break;
                }
                pong_deadline = Some(Instant::now() + PONG_TIMEOUT);
            }
        }
    }

    // Cleanup
    cleanup_host_disconnect(&state, &room_id, &host_id).await;
}

fn handle_host_message(state: &AppState, room_id: &str, host_id: &UserId, text: &str) {
    let msg: HostWebSocketMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("Invalid message from host {}: {}", host_id.as_str(), e);
            return;
        }
    };

    let target_user_id = &msg.user_id;

    // Check if target user is in the room
    if !state.storage.is_user_in_room(room_id, target_user_id) {
        tracing::warn!(
            "Host {} tried to send to user {} who is not in room {}",
            host_id.as_str(),
            target_user_id.as_str(),
            room_id
        );
        return;
    }

    match msg.event.as_str() {
        "MESSAGE" => {
            state.message_bus.send_to_user(
                target_user_id,
                room_id,
                ToUserMessage::message(target_user_id.clone(), msg.message),
            );
        }
        "DISCONNECT" => {
            // Host kicks user
            state.message_bus.send_to_user(
                target_user_id,
                room_id,
                ToUserMessage::disconnect(target_user_id.clone(), DisconnectReason::Kicked),
            );
        }
        other => {
            tracing::warn!("Unknown event '{}' from host {}", other, host_id.as_str());
        }
    }
}

async fn cleanup_host_disconnect(state: &AppState, room_id: &str, host_id: &UserId) {
    tracing::info!(
        "Host {} disconnected from room {}",
        host_id.as_str(),
        room_id
    );

    // Unregister host channel
    state.message_bus.unregister_host(room_id);

    // Get all users and disconnect them
    let users = state.storage.clear_room_users(room_id);
    state
        .message_bus
        .disconnect_room_users(room_id, &users, DisconnectReason::RoomClosed);

    // Remove room
    state.storage.remove_room(room_id);
}
