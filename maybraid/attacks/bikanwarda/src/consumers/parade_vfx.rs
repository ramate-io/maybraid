//! [`ParadeExplosionCue`] → `maybraid-vfx` [`fiery_explosion`](maybraid_vfx::FIERY_EXPLOSION).

use bevy::prelude::*;
use bevy::scene::prelude::bsn;
use maybraid_vfx::{Effect, VfxLibrary, VfxSpawn};

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
		let transform = Transform::from_translation(cue.position)
			.with_scale(Vec3::splat((cue.radius / 1.2).clamp(1.0, 8.0)));
		commands.spawn_scene(bsn! {
			~{transform}
			@{Effect::new(library.fiery_explosion.clone()).with_spawn(VfxSpawn {
				intensity: 1.0,
				playback: 1.0,
				seed: Some(cue.instance.0 ^ crate::cues::entity_seed(cue.owner)),
				..default()
			})}
		});
	}
}
