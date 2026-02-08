use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ToHostEvent {
    JoinRoom,
    LeaveRoom,
    Message,
    Disconnect,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ToUserEvent {
    Message,
    Disconnect,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Event {
    ToHost(ToHostEvent),
    ToUser(ToUserEvent),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DisconnectReason {
    Kicked,
    RoomClosed,
    UserClosed,
    NewConnection,
    PingPong,
}
