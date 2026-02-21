use bevy::{asset::uuid::Uuid, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Component, Serialize, Deserialize)]
pub struct Player(u128);

impl Default for Player {
    fn default() -> Self {
        // Create a UUID for the player.
        let player_id = Uuid::new_v4().as_u128();
        Player(player_id)
    }
}

#[derive(Component, Serialize, Deserialize)]
pub struct Doodad;
