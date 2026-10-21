//! Construct an actor via existing player / character APIs.
//!
//! This does not assign a local player, change cameras, or apply world motor
//! participation. Handoff owns those steps.

use bevy::prelude::*;
use world_player::{
	spawn_player_body, AwaitingTerrainSurface, CharacterLocomotion, CharacterSpecies,
	OffTerrainAnchor, RequestSetCharacter, RequestSetCharacterAppearance,
};

use crate::request::{ActorSpawned, SpawnActor};

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
		commands.entity(actor).insert(AwaitingTerrainSurface);
		if self.interior {
			commands.entity(actor).insert(OffTerrainAnchor { translation: self.position });
		}
		if let Some(appearance) = self.recipe.appearance.clone() {
			commands.spawn(RequestSetCharacterAppearance { appearance });
		} else {
			commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
		}
		ActorSpawned::new(self.identity, actor, self.recipe.dress)
	}
}
