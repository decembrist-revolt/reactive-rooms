use std::fmt;
use super::user::UserId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomId(Uuid);

impl RoomId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for RoomId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RoomType(String);

impl RoomType {
    pub fn new(type_name: impl Into<String>) -> Self {
        Self(type_name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Room {
    pub id: RoomId,
    pub host_id: UserId,
    pub room_type: RoomType,
}

impl Room {
    pub fn new(host_id: UserId, room_type: RoomType) -> Self {
        Self {
            id: RoomId::new(),
            host_id,
            room_type,
        }
    }

    pub fn with_id(id: RoomId, host_id: UserId, room_type: RoomType) -> Self {
        Self {
            id,
            host_id,
            room_type,
        }
    }

    pub fn is_host(&self, user_id: UserId) -> bool {
        self.host_id == user_id
    }
}
