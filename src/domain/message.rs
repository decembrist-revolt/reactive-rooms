use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    event::{DisconnectReason, ToHostEvent, ToUserEvent},
    user::UserId,
};

pub type MessagePayload = Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToHostMessage {
    pub event: ToHostEvent,
    pub user_id: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<MessagePayload>,
}

impl ToHostMessage {
    pub fn join_room(user_id: UserId) -> Self {
        Self {
            event: ToHostEvent::JoinRoom,
            user_id,
            message: None,
        }
    }

    pub fn leave_room(user_id: UserId) -> Self {
        Self {
            event: ToHostEvent::LeaveRoom,
            user_id,
            message: None,
        }
    }

    pub fn message(user_id: UserId, payload: MessagePayload) -> Self {
        Self {
            event: ToHostEvent::Message,
            user_id,
            message: Some(payload),
        }
    }

    pub fn disconnect(user_id: UserId, reason: DisconnectReason) -> Self {
        Self {
            event: ToHostEvent::Disconnect,
            user_id,
            message: Some(serde_json::json!({ "reason": reason })),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToUserMessage {
    pub event: ToUserEvent,
    pub user_id: UserId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<MessagePayload>,
}

impl ToUserMessage {
    pub fn message(user_id: UserId, payload: MessagePayload) -> Self {
        Self {
            event: ToUserEvent::Message,
            user_id,
            message: Some(payload),
        }
    }

    pub fn disconnect(user_id: UserId, reason: DisconnectReason) -> Self {
        Self {
            event: ToUserEvent::Disconnect,
            user_id,
            message: Some(serde_json::json!({ "reason": reason })),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserWebSocketMessage {
    pub event: String,
    pub message: MessagePayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostWebSocketMessage {
    pub event: String,
    pub user_id: UserId,
    pub message: MessagePayload,
}
