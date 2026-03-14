use super::ping::PingTracker;
use crate::{
    AppState,
    domain::{
        event::{DisconnectReason, FromHostEvent, ToHostEvent},
        message::{HostWebSocketMessage, ToUserMessage},
        room::RoomId,
        user::UserId,
    },
};
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

pub async fn handle_host_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    room_id: RoomId,
    host_id: UserId,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut bus_rx = state.message_bus.register_host(&room_id);
    let mut ping = PingTracker::new().await;

    loop {
        tokio::select! {
            msg = bus_rx.recv() => {
                match msg {
                    Some(msg) => {
                        let is_host_disconnect = matches!(msg.event, ToHostEvent::Disconnect)
                            && msg.user_id == host_id;

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

                        if is_host_disconnect {
                            break;
                        }
                    }
                    None => break,
                }
            }

            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        handle_host_message(&state, &room_id, &host_id, &text);
                    }
                    Some(Ok(WsMessage::Pong(_))) => ping.on_pong(),
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error for host {}: {}", host_id.as_str(), e);
                        break;
                    }
                    _ => {}
                }
            }

            _ = ping.interval.tick() => {
                if !ping.on_tick() {
                    tracing::warn!("Host {} pong timeout, disconnecting", host_id.as_str());
                    break;
                }
                if ws_sender.send(WsMessage::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }

    cleanup_host_disconnect(&state, &room_id, &host_id).await;
}

fn handle_host_message(state: &AppState, room_id: &RoomId, host_id: &UserId, text: &str) {
    let msg: HostWebSocketMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("Invalid message from host {}: {}", host_id.as_str(), e);
            return;
        }
    };

    let target_user_id = &msg.user_id;

    if !state.storage.is_user_in_room(room_id, target_user_id) {
        tracing::warn!(
            "Host {} tried to send to user {} who is not in room {}",
            host_id.as_str(),
            target_user_id.as_str(),
            room_id
        );
        return;
    }

    match msg.event {
        FromHostEvent::Message => {
            state.message_bus.send_to_user(
                target_user_id,
                room_id,
                ToUserMessage::message(target_user_id.clone(), msg.message.unwrap_or_default()),
            );
        }
        FromHostEvent::Disconnect => {
            state.message_bus.send_to_user(
                target_user_id,
                room_id,
                ToUserMessage::disconnect(target_user_id.clone(), DisconnectReason::Kicked),
            );
        }
    }
}

async fn cleanup_host_disconnect(state: &AppState, room_id: &RoomId, host_id: &UserId) {
    tracing::info!(
        "Host {} disconnected from room {}",
        host_id.as_str(),
        room_id
    );

    state.message_bus.unregister_host(room_id);

    if let Some((_, users)) = state.storage.remove_room_with_users(room_id) {
        state
            .message_bus
            .disconnect_room_users(room_id, &users, DisconnectReason::RoomClosed);
    }
}
