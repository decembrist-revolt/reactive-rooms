use std::collections::HashSet;

use dashmap::DashMap;

use crate::domain::{
    room::{Room, RoomId},
    user::UserId,
};

#[derive(Clone)]
pub struct RoomStorage {
    rooms: DashMap<String, Room>,
    room_users: DashMap<String, HashSet<UserId>>,
}

impl Default for RoomStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl RoomStorage {
    pub fn new() -> Self {
        Self {
            rooms: DashMap::new(),
            room_users: DashMap::new(),
        }
    }

    pub fn create_room(&self, room: Room) -> Result<RoomId, CreateRoomError> {
        let key = room.id.to_string();
        if self.rooms.contains_key(&key) {
            return Err(CreateRoomError::RoomAlreadyExists);
        }
        let room_id = room.id.clone();
        self.rooms.insert(key.clone(), room);
        self.room_users.insert(key, HashSet::new());
        Ok(room_id)
    }

    pub fn get_room(&self, room_id: &str) -> Option<Room> {
        self.rooms.get(room_id).map(|r| r.clone())
    }

    pub fn remove_room(&self, room_id: &str) -> Option<Room> {
        let room = self.rooms.remove(room_id).map(|(_, r)| r);
        self.room_users.remove(room_id);
        room
    }

    pub fn get_rooms_paginated(&self, page: usize, size: usize) -> (Vec<Room>, usize) {
        let all: Vec<Room> = self.rooms.iter().map(|r| r.value().clone()).collect();
        let total = all.len();
        let start = page * size;
        let rooms = all.into_iter().skip(start).take(size).collect();
        (rooms, total)
    }

    pub fn add_user_to_room(&self, room_id: &str, user_id: UserId) -> bool {
        if let Some(mut users) = self.room_users.get_mut(room_id) {
            users.insert(user_id)
        } else {
            false
        }
    }

    pub fn remove_user_from_room(&self, room_id: &str, user_id: &UserId) {
        if let Some(mut users) = self.room_users.get_mut(room_id) {
            users.remove(user_id);
        }
    }

    pub fn is_user_in_room(&self, room_id: &str, user_id: &UserId) -> bool {
        self.room_users
            .get(room_id)
            .is_some_and(|users| users.contains(user_id))
    }

    pub fn get_room_user_count(&self, room_id: &str) -> usize {
        self.room_users
            .get(room_id)
            .map(|users| users.len())
            .unwrap_or(0)
    }

    pub fn get_room_users(&self, room_id: &str) -> Vec<UserId> {
        self.room_users
            .get(room_id)
            .map(|users| users.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn clear_room_users(&self, room_id: &str) -> Vec<UserId> {
        if let Some(mut users) = self.room_users.get_mut(room_id) {
            let result: Vec<UserId> = users.iter().cloned().collect();
            users.clear();
            result
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug)]
pub enum CreateRoomError {
    RoomAlreadyExists,
}
