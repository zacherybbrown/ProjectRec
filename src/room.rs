use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub public: bool,
}

impl RoomInfo {
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomRegistry {
    pub rooms: Vec<RoomInfo>,
}

impl Default for RoomRegistry {
    fn default() -> Self {
        Self { rooms: Vec::new() }
    }
}

pub fn load_registry() -> Option<RoomRegistry> {
    let contents = fs::read_to_string("rooms.json").ok()?;
    serde_json::from_str(&contents).ok()
}
