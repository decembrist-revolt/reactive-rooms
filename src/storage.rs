use crate::domain::{
    room::{Room, RoomId},
    user::UserId,
};
use dashmap::{DashMap, Entry};
use std::collections::HashSet;

const DEFAULT_MAX_ROOMS: usize = 10_000;

#[derive(Clone)]
pub struct RoomStorage {
    rooms: DashMap<RoomId, Room>,
    room_users: DashMap<RoomId, HashSet<UserId>>,
    max_rooms: usize,
}

impl Default for RoomStorage {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ROOMS)
    }
}

impl RoomStorage {
    pub fn new(max_rooms: usize) -> Self {
        Self {
            rooms: DashMap::new(),
            room_users: DashMap::new(),
            max_rooms,
        }
    }

    pub fn create_room(&self, room: Room) -> Result<RoomId, CreateRoomError> {
        if self.rooms.len() >= self.max_rooms {
            return Err(CreateRoomError::RoomLimitReached);
        }

        let room_id = room.id.clone();
        match self.rooms.entry(room_id.clone()) {
            Entry::Occupied(_) => Err(CreateRoomError::RoomAlreadyExists),
            Entry::Vacant(e) => {
                e.insert(room);
                self.room_users.insert(room_id.clone(), HashSet::new());
                Ok(room_id)
            }
        }
    }

    pub fn get_room(&self, room_id: &RoomId) -> Option<Room> {
        self.rooms.get(room_id).map(|r| r.clone())
    }

    pub fn remove_room_with_users(&self, room_id: &RoomId) -> Option<(Room, Vec<UserId>)> {
        let room = self.rooms.remove(room_id).map(|(_, r)| r)?;
        let users = self
            .room_users
            .remove(room_id)
            .map(|(_, u)| u.into_iter().collect())
            .unwrap_or_default();
        Some((room, users))
    }

    pub fn get_rooms_paginated(&self, page: usize, size: usize) -> (Vec<Room>, usize) {
        let total = self.rooms.len();
        let start = page * size;
        let rooms = self
            .rooms
            .iter()
            .skip(start)
            .take(size)
            .map(|r| r.value().clone())
            .collect();
        (rooms, total)
    }

    pub fn add_user_to_room(&self, room_id: &RoomId, user_id: UserId) -> bool {
        if let Some(mut users) = self.room_users.get_mut(room_id) {
            users.insert(user_id)
        } else {
            false
        }
    }

    pub fn remove_user_from_room(&self, room_id: &RoomId, user_id: &UserId) {
        if let Some(mut users) = self.room_users.get_mut(room_id) {
            users.remove(user_id);
        }
    }

    pub fn is_user_in_room(&self, room_id: &RoomId, user_id: &UserId) -> bool {
        self.room_users
            .get(room_id)
            .is_some_and(|users| users.contains(user_id))
    }

    pub fn get_room_user_count(&self, room_id: &RoomId) -> usize {
        self.room_users
            .get(room_id)
            .map(|users| users.len())
            .unwrap_or(0)
    }
}

#[derive(Debug)]
pub enum CreateRoomError {
    RoomAlreadyExists,
    RoomLimitReached,
}
