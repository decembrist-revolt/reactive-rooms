use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::time::{Instant, interval};

use crate::{
    AppState,
    domain::{
        event::ToUserEvent,
        message::{ToHostMessage, UserWebSocketMessage},
        user::UserId,
    },
};

const PING_INTERVAL: Duration = Duration::from_secs(30);
const PONG_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn handle_user_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    room_id: String,
    user_id: UserId,
) {
    // Register user in room and message bus
    state.storage.add_user_to_room(&room_id, user_id.clone());
    let mut bus_rx = state.message_bus.register_user(&user_id, &room_id);

    // Notify host of user join
    state
        .message_bus
        .send_to_host(&room_id, ToHostMessage::join_room(user_id.clone()));

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut ping_interval = interval(PING_INTERVAL);
    ping_interval.tick().await; // consume first immediate tick
    let mut pong_deadline: Option<Instant> = None;

    loop {
        tokio::select! {
            // Message from host via bus -> forward to user WS
            msg = bus_rx.recv() => {
                match msg {
                    Some(msg) => {
                        let is_disconnect = matches!(msg.event, ToUserEvent::Disconnect);

                        match serde_json::to_string(&msg) {
                            Ok(json) => {
                                let _ = ws_sender.send(WsMessage::Text(json.into())).await;
                            }
                            Err(e) => {
                                tracing::error!("Failed to serialize message for user {}: {}", user_id.as_str(), e);
                            }
                        }

                        if is_disconnect {
                            break;
                        }
                    }
                    None => {
                        // Channel closed (host disconnected / room closed)
                        break;
                    }
                }
            }

            // Message from user WS -> route to host
            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        handle_user_message(&state, &room_id, &user_id, &text);
                    }
                    Some(Ok(WsMessage::Pong(_))) => {
                        pong_deadline = None;
                    }
                    Some(Ok(WsMessage::Close(_))) | None => {
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error for user {}: {}", user_id.as_str(), e);
                        break;
                    }
                    _ => {}
                }
            }

            // Ping tick
            _ = ping_interval.tick() => {
                if let Some(deadline) = pong_deadline
                    && Instant::now() > deadline {
                        tracing::warn!("User {} pong timeout, disconnecting", user_id.as_str());
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
    cleanup_user_disconnect(&state, &room_id, &user_id).await;
}

fn handle_user_message(
    state: &AppState,
    room_id: &str,
    user_id: &UserId,
    text: &str,
) {
    let msg: UserWebSocketMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!(
                "Invalid message from user {}: {}",
                user_id.as_str(),
                e
            );
            return;
        }
    };

    match msg.event.as_str() {
        "MESSAGE" => {
            state.message_bus.send_to_host(
                room_id,
                ToHostMessage::message(user_id.clone(), msg.message),
            );
        }
        other => {
            tracing::warn!(
                "Unknown event '{}' from user {}",
                other,
                user_id.as_str()
            );
        }
    }
}

async fn cleanup_user_disconnect(
    state: &AppState,
    room_id: &str,
    user_id: &UserId,
) {
    tracing::info!("User {} disconnected from room {}", user_id.as_str(), room_id);

    // Remove user from room
    state.storage.remove_user_from_room(room_id, user_id);

    // Unregister user channel
    state.message_bus.unregister_user(user_id, room_id);

    // Notify host that user left
    state
        .message_bus
        .send_to_host(room_id, ToHostMessage::leave_room(user_id.clone()));
}
