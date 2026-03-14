use std::collections::HashMap;

use protocol::{InputCommand, PlayerId, PlayerSnapshot, Vec2, WorldSnapshot};

pub const TICK_HZ: u16 = 20;
pub const TICK_DT_SECS: f32 = 1.0 / TICK_HZ as f32;
pub const PLAYER_SPEED_UNITS_PER_SEC: f32 = 4.0;

#[derive(Debug, Clone)]
pub struct PlayerState {
    pub name: String,
    pub position: Vec2,
    pub last_processed_input: u32,
}

#[derive(Debug, Default)]
pub struct GameWorld {
    tick: u64,
    players: HashMap<PlayerId, PlayerState>,
}

impl GameWorld {
    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn advance_tick(&mut self) {
        self.tick += 1;
    }

    pub fn add_player(&mut self, player_id: PlayerId, name: String) {
        self.players.insert(
            player_id,
            PlayerState {
                name,
                position: Vec2::ZERO,
                last_processed_input: 0,
            },
        );
    }

    pub fn remove_player(&mut self, player_id: PlayerId) {
        self.players.remove(&player_id);
    }

    pub fn apply_input(&mut self, player_id: PlayerId, input: &InputCommand, dt_secs: f32) {
        let Some(player) = self.players.get_mut(&player_id) else {
            return;
        };

        let mut dx = input.move_x;
        let mut dy = input.move_y;
        let len = (dx * dx + dy * dy).sqrt();

        if len > 1.0 {
            dx /= len;
            dy /= len;
        }

        player.position.x += dx * PLAYER_SPEED_UNITS_PER_SEC * dt_secs;
        player.position.y += dy * PLAYER_SPEED_UNITS_PER_SEC * dt_secs;
        player.last_processed_input = input.seq;
    }

    pub fn snapshot(&self) -> WorldSnapshot {
        let mut players: Vec<_> = self
            .players
            .iter()
            .map(|(id, state)| PlayerSnapshot {
                id: *id,
                name: state.name.clone(),
                position: state.position,
                last_processed_input: state.last_processed_input,
            })
            .collect();

        players.sort_by_key(|p| p.id);

        WorldSnapshot {
            tick: self.tick,
            players,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagonal_input_is_normalized() {
        let mut world = GameWorld::default();
        world.add_player(1, "p1".to_string());

        world.apply_input(
            1,
            &InputCommand {
                seq: 1,
                move_x: 1.0,
                move_y: 1.0,
            },
            1.0,
        );

        let snapshot = world.snapshot();
        let player = &snapshot.players[0];
        let moved =
            (player.position.x * player.position.x + player.position.y * player.position.y).sqrt();

        assert!((moved - PLAYER_SPEED_UNITS_PER_SEC).abs() < 1e-4);
    }
}
