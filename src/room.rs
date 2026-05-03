use serde::{Deserialize, Serialize};

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
