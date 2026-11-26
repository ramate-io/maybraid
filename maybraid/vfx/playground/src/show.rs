//! Apply queued `/show` requests through the crate spawn helper.

use bevy::prelude::*;
use game_commands::ui::GameCommandStatusText;
use maybraid_vfx::{canonicalize_effect_name, VfxLibrary, DUST, FIERY_EXPLOSION};

use crate::inspector::PreviewInspector;
use crate::preview::{PreviewEffect, PreviewRecipes, PreviewSelection};

use crate::commands::{LoopEnabled, PendingVfxShows, PreviewFreeze, SpreadOnce, VfxShowRequest};

pub const SHOW_ORIGIN: Vec3 = Vec3::new(0.0, 1.2, 0.0);

#[derive(Resource, Default, Clone, Debug)]
pub struct LastVfxShow {
	pub label: String,
	pub request: Option<VfxShowRequest>,
}

/// Replay the last `/show` so a one-shot burst can be judged.
#[derive(Resource, Default, Clone, Debug)]
pub struct RepeatingVfxShow {
	pub request: Option<VfxShowRequest>,
	pub wait: f32,
	pub root: Option<Entity>,
}

pub fn apply_pending_shows(
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
	registry: Res<PreviewRecipes>,
	selection: Res<PreviewSelection>,
	mut inspector: ResMut<PreviewInspector>,
	mut pending: ResMut<PendingVfxShows>,
	mut last: ResMut<LastVfxShow>,
	mut repeating: ResMut<RepeatingVfxShow>,
	mut spread: ResMut<SpreadOnce>,
	mut status: ResMut<GameCommandStatusText>,
	looping: Res<LoopEnabled>,
) {
	let Some(library) = library else {
		return;
	};
	if spread.0 {
		spread.0 = false;
		if let Some(base) = last.request.clone() {
			pending.0.extend([
				VfxShowRequest { distance: 0.0, seed: base.seed, ..base.clone() },
				VfxShowRequest {
					distance: 6.0,
					seed: base.seed.map(|s| s.wrapping_add(1)),
					..base.clone()
				},
				VfxShowRequest {
					distance: 14.0,
					seed: base.seed.map(|s| s.wrapping_add(2)),
					..base
				},
			]);
		}
	}
	for request in pending.0.drain(..) {
		let Some(preview) = preview(&registry, &library, &request.effect) else {
			last.label = format!(
				"unknown effect `{}`; registered previews: {}",
				request.effect,
				registry.names().collect::<Vec<_>>().join(", ")
			);
			continue;
		};
		let name = preview.definition.name.clone();
		let (root, spawn) = preview.spawn(&mut commands, &request, &selection, &mut inspector);
		if looping.0 {
			repeating.request = Some(VfxShowRequest { seed: spawn.seed, ..request.clone() });
			repeating.wait = 0.0;
			repeating.root = Some(root);
		}
		last.request = Some(VfxShowRequest { seed: spawn.seed, ..request.clone() });
		last.label = format!(
			"{} · scale {:.2} · intensity {:.2} · playback {:.2} · d {:.1} · seed {}",
			name,
			request.scale,
			request.intensity,
			request.playback,
			request.distance,
			spawn.resolved_seed()
		);
		status.0 = format!("show {}", last.label);
	}
}

pub fn repeat_shown_effect(
	mut commands: Commands,
	time: Res<Time>,
	library: Option<Res<VfxLibrary>>,
	registry: Res<PreviewRecipes>,
	selection: Res<PreviewSelection>,
	mut inspector: ResMut<PreviewInspector>,
	looping: Res<LoopEnabled>,
	freeze: Res<PreviewFreeze>,
	mut repeating: ResMut<RepeatingVfxShow>,
	instances: Query<(), With<maybraid_vfx::VfxInstance>>,
) {
	if !looping.0 || freeze.age.is_some() {
		return;
	}
	let Some(library) = library else {
		return;
	};
	let Some(request) = repeating.request.clone() else {
		return;
	};

	// Completion follows actual readiness and manifest timing, including delayed layers.
	if repeating.root.is_some_and(|root| instances.contains(root)) {
		return;
	}
	repeating.wait += time.delta_secs();
	if repeating.wait >= 0.45 {
		let Some(preview) = preview(&registry, &library, &request.effect) else {
			return;
		};
		repeating.wait = 0.0;
		repeating.root = Some(preview.spawn(&mut commands, &request, &selection, &mut inspector).0);
	}
}

pub fn pause_at_frozen_age(
	freeze: Res<PreviewFreeze>,
	instances: Query<&maybraid_vfx::VfxInstance>,
	mut time: ResMut<Time<Virtual>>,
) {
	let Some(target) = freeze.age else {
		return;
	};
	if time.is_paused() {
		return;
	}
	if instances.iter().any(|instance| instance.armed && instance.age + 1e-3 >= target) {
		time.pause();
	}
}

fn preview(registry: &PreviewRecipes, library: &VfxLibrary, name: &str) -> Option<PreviewEffect> {
	registry.build(name).or_else(|| match canonicalize_effect_name(name) {
		Some(DUST) => Some(PreviewEffect::typed(library.dust.clone())),
		Some(FIERY_EXPLOSION) => Some(PreviewEffect::typed(library.fiery_explosion.clone())),
		_ => library.get(name).map(PreviewEffect::definition),
	})
}
