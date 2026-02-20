use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    #[serde(rename = "type")]
    pub room_type: String,
    #[serde(rename = "hostId")]
    pub host_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoomResponse {
    pub room_id: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomWithPlayerCount {
    pub room_id: String,
    pub host_id: String,
    #[serde(rename = "type")]
    pub room_type: String,
    pub player_count: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomsPageResponse {
    pub rooms: Vec<RoomWithPlayerCount>,
    pub total_rooms: usize,
    pub page: usize,
    pub size: usize,
}

#[derive(Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub size: Option<usize>,
}

#[derive(Deserialize)]
pub struct WsQueryParams {
    #[serde(rename = "roomId")]
    pub room_id: String,
    #[serde(rename = "type")]
    pub connection_type: String,
}
