use dashmap::DashMap;
use tokio::sync::mpsc;

use crate::domain::{
    event::DisconnectReason,
    message::{ToHostMessage, ToUserMessage},
    user::UserId,
};

const CHANNEL_BUFFER: usize = 256;

#[derive(Clone)]
pub struct MessageBus {
    /// roomId (string) -> sender for messages to host
    host_channels: DashMap<String, mpsc::Sender<ToHostMessage>>,
    /// "userId:roomId" -> sender for messages to user
    user_channels: DashMap<String, mpsc::Sender<ToUserMessage>>,
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            host_channels: DashMap::new(),
            user_channels: DashMap::new(),
        }
    }

    pub fn register_host(&self, room_id: &str) -> mpsc::Receiver<ToHostMessage> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        self.host_channels.insert(room_id.to_string(), tx);
        rx
    }

    pub fn unregister_host(&self, room_id: &str) {
        self.host_channels.remove(room_id);
    }

    pub fn send_to_host(&self, room_id: &str, msg: ToHostMessage) {
        if let Some(tx) = self.host_channels.get(room_id) {
            let _ = tx.try_send(msg);
        }
    }

    /// Register a user channel. If the user already has a connection,
    /// send a Disconnect(NewConnection) to the old channel first.
    pub fn register_user(&self, user_id: &UserId, room_id: &str) -> mpsc::Receiver<ToUserMessage> {
        let key = user_channel_key(user_id, room_id);
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);

        if let Some(old_tx) = self.user_channels.insert(key, tx) {
            let _ = old_tx.try_send(ToUserMessage::disconnect(
                user_id.clone(),
                DisconnectReason::NewConnection,
            ));
        }

        rx
    }

    pub fn unregister_user(&self, user_id: &UserId, room_id: &str) {
        let key = user_channel_key(user_id, room_id);
        self.user_channels.remove(&key);
    }

    pub fn send_to_user(&self, user_id: &UserId, room_id: &str, msg: ToUserMessage) {
        let key = user_channel_key(user_id, room_id);
        if let Some(tx) = self.user_channels.get(&key) {
            let _ = tx.try_send(msg);
        }
    }

    /// Disconnect all users in a room by sending Disconnect messages
    pub fn disconnect_room_users(
        &self,
        room_id: &str,
        user_ids: &[UserId],
        reason: DisconnectReason,
    ) {
        for user_id in user_ids {
            self.send_to_user(
                user_id,
                room_id,
                ToUserMessage::disconnect(user_id.clone(), reason.clone()),
            );
        }
    }

    /// Disconnect the host of a room
    pub fn disconnect_host(&self, room_id: &str, host_id: &UserId, reason: DisconnectReason) {
        self.send_to_host(room_id, ToHostMessage::disconnect(host_id.clone(), reason));
    }
}

fn user_channel_key(user_id: &UserId, room_id: &str) -> String {
    format!("{}:{}", user_id.as_str(), room_id)
}
