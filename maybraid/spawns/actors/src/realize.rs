//! Construct an actor via existing player / character APIs.
//!
//! The body is created at the accepted pose with zero velocity and live
//! gravity. This does not assign a local player, change cameras, or apply
//! world motor participation. Handoff owns those steps.

use avian3d::prelude::{AngularVelocity, GravityScale, LinearVelocity};
use bevy::prelude::*;
use world_player::{
	spawn_player_body, CharacterLocomotion, CharacterSpecies, RequestSetCharacter,
	RequestSetCharacterAppearance,
};

use crate::request::{ActorSpawned, SpawnActor};

const PLAY_GRAVITY_SCALE: f32 = 1.25;

impl SpawnActor {
	/// Build the body at the resolved pose and queue character appearance.
	pub fn realize(
		&self,
		commands: &mut Commands,
		meshes: &mut Assets<Mesh>,
		materials: &mut Assets<StandardMaterial>,
		locomotion: &CharacterLocomotion,
	) -> ActorSpawned {
		let actor = spawn_player_body(commands, meshes, materials, locomotion, self.position);
		commands.entity(actor).insert((
			LinearVelocity::ZERO,
			AngularVelocity::ZERO,
			GravityScale(PLAY_GRAVITY_SCALE),
		));
		if let Some(appearance) = self.appearance.clone() {
			commands.spawn(RequestSetCharacterAppearance { appearance });
		} else {
			commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
		}
		ActorSpawned::new(self.identity, self.attempt, actor, self.dress)
	}
}
