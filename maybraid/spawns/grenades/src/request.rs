//! Thrown-grenade spawn request. The payload is complete; the interpreter
//! does not rediscover the equipped item or current player.

pub use grenades::{DropLiveGrenade, SpawnThrownGrenade, ThrownGrenadeSpawn};

use bevy::prelude::*;

/// Simulation body created from a throw or drop.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrenadeSpawned {
	pub grenade: Entity,
	pub source: Entity,
}

impl GrenadeSpawned {
	pub const fn new(grenade: Entity, source: Entity) -> Self {
		Self { grenade, source }
	}
}
