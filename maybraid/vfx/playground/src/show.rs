//! Apply queued `/show` requests through the crate spawn helper.

use bevy::prelude::*;
use game_commands::ui::GameCommandStatusText;
use maybraid_vfx::{spawn_vfx, VfxLibrary, VfxSpawn};

use crate::commands::PendingVfxShows;

pub const SHOW_ORIGIN: Vec3 = Vec3::new(0.0, 1.2, 0.0);

#[derive(Resource, Default, Clone, Debug)]
pub struct LastVfxShow {
	pub label: String,
}

pub fn apply_pending_shows(
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
	mut pending: ResMut<PendingVfxShows>,
	mut last: ResMut<LastVfxShow>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let Some(library) = library else {
		return;
	};
	for request in pending.0.drain(..) {
		let Some(definition) = library.get(&request.effect) else {
			continue;
		};
		spawn_vfx(
			&mut commands,
			definition,
			VfxSpawn {
				transform: Transform::from_translation(SHOW_ORIGIN),
				scale: request.scale,
				intensity: request.intensity,
				..default()
			},
		);
		last.label = format!(
			"{} · scale {:.2} · intensity {:.2}",
			definition.name, request.scale, request.intensity
		);
		status.0 = format!("show {}", last.label);
	}
}
