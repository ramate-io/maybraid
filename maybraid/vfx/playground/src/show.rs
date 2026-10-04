//! Apply queued `/show` requests through the crate spawn helper.

use bevy::prelude::*;
use game_commands::ui::GameCommandStatusText;
use maybraid_vfx::{spawn_vfx, VfxLibrary, VfxSpawn};

use crate::commands::{PendingVfxShows, VfxShowRequest};

pub const SHOW_ORIGIN: Vec3 = Vec3::new(0.0, 1.2, 0.0);

#[derive(Resource, Default, Clone, Debug)]
pub struct LastVfxShow {
	pub label: String,
}

/// Replay the last `/show` so a one-shot burst can be judged.
#[derive(Resource, Default, Clone, Debug)]
pub struct RepeatingVfxShow {
	pub request: Option<VfxShowRequest>,
	pub wait: f32,
}

pub fn apply_pending_shows(
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
	mut pending: ResMut<PendingVfxShows>,
	mut last: ResMut<LastVfxShow>,
	mut repeating: ResMut<RepeatingVfxShow>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let Some(library) = library else {
		return;
	};
	for request in pending.0.drain(..) {
		let Some(definition) = library.get(&request.effect) else {
			continue;
		};
		burst(&mut commands, definition, &request);
		repeating.request = Some(request.clone());
		repeating.wait = 0.0;
		last.label = format!(
			"{} · scale {:.2} · intensity {:.2} · looping",
			definition.name, request.scale, request.intensity
		);
		status.0 = format!("show {}", last.label);
	}
}

pub fn repeat_shown_effect(
	mut commands: Commands,
	time: Res<Time>,
	library: Option<Res<VfxLibrary>>,
	mut repeating: ResMut<RepeatingVfxShow>,
) {
	let Some(library) = library else {
		return;
	};
	let Some(request) = repeating.request.clone() else {
		return;
	};
	let Some(definition) = library.get(&request.effect) else {
		return;
	};
	repeating.wait += time.delta_secs();
	if repeating.wait >= definition.duration() + 0.45 {
		repeating.wait = 0.0;
		burst(&mut commands, definition, &request);
	}
}

fn burst(
	commands: &mut Commands,
	definition: &maybraid_vfx::EffectDefinition,
	request: &VfxShowRequest,
) {
	spawn_vfx(
		commands,
		definition,
		VfxSpawn {
			transform: Transform::from_translation(SHOW_ORIGIN),
			scale: request.scale,
			intensity: request.intensity,
			..default()
		},
	);
}
