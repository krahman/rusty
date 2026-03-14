use serde::{Deserialize, Serialize};

pub type PlayerId = u64;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputCommand {
    pub seq: u32,
    pub move_x: f32,
    pub move_y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Join { name: String },
    Input(InputCommand),
    Ping { client_time_ms: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSnapshot {
    pub id: PlayerId,
    pub name: String,
    pub position: Vec2,
    pub last_processed_input: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub tick: u64,
    pub players: Vec<PlayerSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome { player_id: PlayerId, tick_hz: u16 },
    Snapshot(WorldSnapshot),
    Pong { client_time_ms: u64 },
    Error { message: String },
}
