//! [`ParadeExplosionCue`] → `maybraid-vfx` [`fiery_explosion`](maybraid_vfx::FIERY_EXPLOSION).

use bevy::prelude::*;
use maybraid_vfx::{SpawnVfxExt, VfxLibrary, VfxSpawn};

use crate::consumers::BikanwardaPresentationSystems;
use crate::ParadeExplosionCue;

pub fn plugin(app: &mut App) {
	app.add_systems(
		Update,
		consume_parade_explosions.in_set(BikanwardaPresentationSystems::Consume),
	);
}

fn consume_parade_explosions(
	mut cues: MessageReader<ParadeExplosionCue>,
	library: Option<Res<VfxLibrary>>,
	mut commands: Commands,
) {
	let Some(library) = library else {
		return;
	};
	for cue in cues.read() {
		commands.spawn_vfx(
			&library.fiery_explosion,
			VfxSpawn {
				transform: Transform::from_translation(cue.position),
				scale: (cue.radius / 1.2).clamp(1.0, 8.0),
				intensity: 1.0,
				playback: 1.0,
				seed: Some(cue.instance.0 ^ crate::cues::entity_seed(cue.owner)),
				..default()
			},
		);
	}
}
