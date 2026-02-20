use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToHostEvent {
    JoinRoom,
    LeaveRoom,
    Message,
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToUserEvent {
    Message,
    Disconnect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisconnectReason {
    Kicked,
    RoomClosed,
    UserClosed,
    NewConnection,
    PingPong,
}
