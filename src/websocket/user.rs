use super::ping::PingTracker;
use crate::{
    AppState,
    domain::{
        event::{FromUserEvent, ToUserEvent},
        message::{ToHostMessage, UserWebSocketMessage},
        room::RoomId,
        user::UserId,
    },
};
use axum::extract::ws::{Message as WsMessage, WebSocket};
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

pub async fn handle_user_ws(
    socket: WebSocket,
    state: Arc<AppState>,
    room_id: RoomId,
    user_id: UserId,
) {
    state.storage.add_user_to_room(&room_id, user_id.clone());
    let mut bus_rx = state.message_bus.register_user(&user_id, &room_id);

    state
        .message_bus
        .send_to_host(&room_id, ToHostMessage::join_room(user_id.clone()));

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut ping = PingTracker::new().await;

    loop {
        tokio::select! {
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
                    None => break,
                }
            }

            ws_msg = ws_receiver.next() => {
                match ws_msg {
                    Some(Ok(WsMessage::Text(text))) => {
                        handle_user_message(&state, &room_id, &user_id, &text);
                    }
                    Some(Ok(WsMessage::Pong(_))) => ping.on_pong(),
                    Some(Ok(WsMessage::Close(_))) | None => break,
                    Some(Err(e)) => {
                        tracing::error!("WebSocket error for user {}: {}", user_id.as_str(), e);
                        break;
                    }
                    _ => {}
                }
            }

            _ = ping.interval.tick() => {
                if !ping.on_tick() {
                    tracing::warn!("User {} pong timeout, disconnecting", user_id.as_str());
                    break;
                }
                if ws_sender.send(WsMessage::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }

    cleanup_user_disconnect(&state, &room_id, &user_id).await;
}

fn handle_user_message(state: &AppState, room_id: &RoomId, user_id: &UserId, text: &str) {
    let msg: UserWebSocketMessage = match serde_json::from_str(text) {
        Ok(msg) => msg,
        Err(e) => {
            tracing::warn!("Invalid message from user {}: {}", user_id.as_str(), e);
            return;
        }
    };

    match msg.event {
        FromUserEvent::Message => {
            state.message_bus.send_to_host(
                room_id,
                ToHostMessage::message(user_id.clone(), msg.message),
            );
        }
    }
}

async fn cleanup_user_disconnect(state: &AppState, room_id: &RoomId, user_id: &UserId) {
    tracing::info!(
        "User {} disconnected from room {}",
        user_id.as_str(),
        room_id
    );

    state.storage.remove_user_from_room(room_id, user_id);
    state.message_bus.unregister_user(user_id, room_id);
    state
        .message_bus
        .send_to_host(room_id, ToHostMessage::leave_room(user_id.clone()));
}
