use crate::domain::{
    event::DisconnectReason,
    message::{ToHostMessage, ToUserMessage},
    room::RoomId,
    user::UserId,
};
use dashmap::DashMap;
use tokio::sync::mpsc;

const CHANNEL_BUFFER: usize = 256;

#[derive(Clone, PartialEq, Eq, Hash)]
struct UserChannelKey {
    user_id: UserId,
    room_id: RoomId,
}

#[derive(Clone)]
pub struct MessageBus {
    host_channels: DashMap<RoomId, mpsc::Sender<ToHostMessage>>,
    user_channels: DashMap<UserChannelKey, mpsc::Sender<ToUserMessage>>,
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

    pub fn register_host(&self, room_id: &RoomId) -> mpsc::Receiver<ToHostMessage> {
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);
        self.host_channels.insert(room_id.clone(), tx);
        rx
    }

    pub fn unregister_host(&self, room_id: &RoomId) {
        self.host_channels.remove(room_id);
    }

    pub fn send_to_host(&self, room_id: &RoomId, msg: ToHostMessage) {
        if let Some(tx) = self.host_channels.get(room_id)
            && let Err(e) = tx.try_send(msg)
        {
            tracing::warn!("Failed to send message to host in room {}: {}", room_id, e);
        }
    }

    pub fn register_user(
        &self,
        user_id: &UserId,
        room_id: &RoomId,
    ) -> mpsc::Receiver<ToUserMessage> {
        let key = UserChannelKey {
            user_id: user_id.clone(),
            room_id: room_id.clone(),
        };
        let (tx, rx) = mpsc::channel(CHANNEL_BUFFER);

        if let Some(old_tx) = self.user_channels.insert(key, tx)
            && let Err(e) = old_tx.try_send(ToUserMessage::disconnect(
                user_id.clone(),
                DisconnectReason::NewConnection,
            ))
        {
            tracing::warn!(
                "Failed to send disconnect to replaced user {}: {}",
                user_id.as_str(),
                e
            );
        }

        rx
    }

    pub fn unregister_user(&self, user_id: &UserId, room_id: &RoomId) {
        let key = UserChannelKey {
            user_id: user_id.clone(),
            room_id: room_id.clone(),
        };
        self.user_channels.remove(&key);
    }

    pub fn send_to_user(&self, user_id: &UserId, room_id: &RoomId, msg: ToUserMessage) {
        let key = UserChannelKey {
            user_id: user_id.clone(),
            room_id: room_id.clone(),
        };
        if let Some(tx) = self.user_channels.get(&key)
            && let Err(e) = tx.try_send(msg)
        {
            tracing::warn!(
                "Failed to send message to user {} in room {}: {}",
                user_id.as_str(),
                room_id,
                e
            );
        }
    }

    pub fn disconnect_room_users(
        &self,
        room_id: &RoomId,
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

    pub fn disconnect_host(&self, room_id: &RoomId, host_id: &UserId, reason: DisconnectReason) {
        self.send_to_host(room_id, ToHostMessage::disconnect(host_id.clone(), reason));
    }
}
