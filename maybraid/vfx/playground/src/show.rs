//! Apply queued `/show` requests through the crate spawn helper.

use bevy::prelude::*;
use game_commands::ui::GameCommandStatusText;
use maybraid_vfx::{
	canonicalize_effect_name, spawn_vfx, Effect, VfxLibrary, VfxSpawn, DUST, FIERY_EXPLOSION,
};

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
}

pub fn apply_pending_shows(
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
	mut pending: ResMut<PendingVfxShows>,
	mut last: ResMut<LastVfxShow>,
	mut repeating: ResMut<RepeatingVfxShow>,
	mut spread: ResMut<SpreadOnce>,
	mut status: ResMut<GameCommandStatusText>,
	looping: Res<LoopEnabled>,
	herds: Query<Entity, With<crate::herd::HerdActors>>,
	mut last_herd: ResMut<crate::herd::LastHerd>,
	comparison: Option<Res<crate::dust_comparison::Comparison>>,
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
	if !pending.0.is_empty() {
		for root in &herds {
			commands.entity(root).try_despawn();
		}
		last_herd.0 = None;
	}
	for mut request in pending.0.drain(..) {
		let original =
			crate::dust_comparison::selection(&mut commands, &mut request, comparison.as_deref());
		let Some(mut definition) = library.get(&request.effect) else {
			continue;
		};
		configure_explosion(&mut definition, &request);
		let authored = matches!(
			request.effect.as_str(),
			"cartoon-explosion" | "combined-explosion" | "explosion-compare"
		);
		commands.queue(move |w: &mut World| crate::explosion_preview::scene(w, authored));
		let spawn = burst(&mut commands, &library, &definition, &request);
		if looping.0 && crate::dust_comparison::pair(&original.effect).is_none() {
			repeating.request = Some(VfxShowRequest { seed: spawn.seed, ..request.clone() });
			repeating.wait = 0.0;
		}
		last.request = Some(VfxShowRequest { seed: spawn.seed, ..original });
		last.label = format!(
			"{} · scale {:.2} · intensity {:.2} · playback {:.2} · d {:.1} · seed {}",
			definition.name,
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
	looping: Res<LoopEnabled>,
	freeze: Res<PreviewFreeze>,
	mut repeating: ResMut<RepeatingVfxShow>,
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
	let Some(mut definition) = library.get(&request.effect) else {
		return;
	};
	repeating.wait += time.delta_secs();
	let playback = request.playback.clamp(maybraid_vfx::MIN_PLAYBACK, maybraid_vfx::MAX_PLAYBACK);
	if repeating.wait >= definition.duration() / playback + 0.45 {
		repeating.wait = 0.0;
		configure_explosion(&mut definition, &request);
		burst(&mut commands, &library, &definition, &request);
	}
}

// Inspection only: clamp the host target before the fixed-tick part catches up.
pub fn clamp_frozen_age(
	freeze: Res<PreviewFreeze>,
	mut instances: Query<&mut maybraid_vfx::VfxInstance>,
) {
	if let Some(target) = freeze.age {
		for mut i in &mut instances {
			i.age = i.age.min(target);
		}
	}
}
pub fn pause_at_frozen_age(
	freeze: Res<PreviewFreeze>,
	instances: Query<&maybraid_vfx::VfxInstance>,
	states: Query<&maybraid_vfx::parts::combined::CombinedState>,
	cartoon: Query<&maybraid_vfx::parts::cartoon_flow::CartoonFlow>,
	mut time: ResMut<Time<Virtual>>,
) {
	let Some(target) = freeze.age else {
		return;
	};
	if time.is_paused() {
		return;
	}
	if instances.iter().any(|instance| instance.armed && instance.age >= target)
		&& states.iter().all(|s| s.simulation.time() + 1. / 120. >= target as f64)
		&& cartoon.iter().all(|s| s.tick as f64 / 120. + 1. / 120. >= target as f64)
	{
		time.pause();
	}
}

fn burst(
	commands: &mut Commands,
	library: &VfxLibrary,
	definition: &maybraid_vfx::EffectDefinition,
	request: &VfxShowRequest,
) -> VfxSpawn {
	let origin = preview_origin(&definition.name);
	let spawn = VfxSpawn {
		transform: Transform::from_translation(origin + Vec3::Z * request.distance),
		scale: request.scale,
		intensity: request.intensity,
		seed: request.seed,
		playback: request.playback,
		..default()
	}
	.resolved_for(definition);
	match canonicalize_effect_name(&request.effect) {
		Some("combined-dust" | "combined-wave") => {
			commands.spawn_scene(Effect { recipe: library.combined.clone(), spawn: spawn.clone() });
		}
		Some("combined-landing") => {
			commands.spawn_scene(Effect {
				recipe: library.combined_landing.clone(),
				spawn: spawn.clone(),
			});
		}
		Some("combined-landing-contact") => {
			commands.spawn_scene(Effect {
				recipe: library.combined_landing_contact.clone(),
				spawn: spawn.clone(),
			});
		}
		Some("combined-wave-contact") => {
			commands.spawn_scene(Effect {
				recipe: library.combined_wave_contact.clone(),
				spawn: spawn.clone(),
			});
		}
		Some("cartoon-wave") => {
			commands
				.spawn_scene(Effect { recipe: library.cartoon_wave.clone(), spawn: spawn.clone() });
		}
		Some("cartoon-landing") => {
			commands.spawn_scene(Effect {
				recipe: library.cartoon_landing.clone(),
				spawn: spawn.clone(),
			});
		}
		Some(DUST) => {
			commands.spawn_scene(Effect { recipe: library.dust.clone(), spawn: spawn.clone() });
		}
		Some(FIERY_EXPLOSION) => {
			commands.spawn_scene(Effect {
				recipe: library.fiery_explosion.clone(),
				spawn: spawn.clone(),
			});
		}
		_ => {
			spawn_vfx(commands, definition, spawn.clone());
		}
	}
	spawn
}

#[cfg(test)]
mod freeze_tests {
	use super::*;
	#[test]
	fn inspection_clamps_age_without_changing_normal_playback() -> Result<(), String> {
		let mut app = App::new();
		app.init_resource::<PreviewFreeze>().add_systems(Update, clamp_frozen_age);
		let id = app
			.world_mut()
			.spawn(maybraid_vfx::VfxInstance {
				name: "flow-guided-dust".into(),
				age: 3.02,
				duration: 14.,
				playback: 1.,
				seed: 9271,
				armed: true,
			})
			.id();
		app.update();
		assert_eq!(app.world().get::<maybraid_vfx::VfxInstance>(id).ok_or("instance")?.age, 3.02);
		app.world_mut().resource_mut::<PreviewFreeze>().age = Some(3.);
		app.update();
		assert_eq!(app.world().get::<maybraid_vfx::VfxInstance>(id).ok_or("instance")?.age, 3.);
		Ok(())
	}
}

fn configure_explosion(d: &mut maybraid_vfx::EffectDefinition, r: &VfxShowRequest) {
	use maybraid_vfx::composition::{
		EffectPart, ExplosionDiagnostic, ExplosionLod, ExplosionQuality,
	};
	let heading = r.explosion.heading.map(|a| {
		let a = a.to_radians();
		Vec3::new(a.cos(), 0., a.sin())
	});
	for l in &mut d.layers {
		match &mut l.part {
			EffectPart::CartoonExplosion(p) => {
				p.heading = heading;
				p.lod = match r.explosion.lod.as_str() {
					"medium" => ExplosionLod::Medium,
					"low" => ExplosionLod::Low,
					_ => ExplosionLod::Full,
				};
			}
			EffectPart::CombinedExplosion(p) => {
				p.heading = heading;
				p.quality = match r.explosion.quality.as_str() {
					"medium" => ExplosionQuality::Medium,
					"low" => ExplosionQuality::Low,
					_ => ExplosionQuality::High,
				};
				p.diagnostic = match r.explosion.diagnostic.as_str() {
					"main" => ExplosionDiagnostic::Main,
					"fine" => ExplosionDiagnostic::Fine,
					"spikes" => ExplosionDiagnostic::Spikes,
					"section" => ExplosionDiagnostic::Section,
					_ => ExplosionDiagnostic::Composed,
				};
			}
			_ => {}
		}
	}
}

fn preview_origin(name: &str) -> Vec3 {
	if matches!(name, "cartoon-explosion" | "combined-explosion")
		|| name == "explosion-compare"
		|| name == DUST
		|| ((name == "cartoon-landing"
			|| name == "cartoon-wave-contact"
			|| name == "cartoon-landing-contact"
			|| name == "cartoon-wave-interactive"
			|| name == "cartoon-landing-interactive")
			|| name == "cartoon-wave"
			|| matches!(
				name,
				"combined-landing"
					| "combined-wave"
					| "combined-landing-contact"
					| "combined-wave-contact"
			) || name == "combined-dust"
			|| (name == "interactive-dust" || name == "flow-guided-dust"))
	{
		Vec3::ZERO
	} else {
		SHOW_ORIGIN
	}
}

#[cfg(test)]
mod explosion_origin_tests {
	#[test]
	fn preserves_fiery_placement() {
		assert_eq!(super::preview_origin(maybraid_vfx::FIERY_EXPLOSION), super::SHOW_ORIGIN);
		assert_eq!(super::preview_origin("cartoon-explosion"), bevy::prelude::Vec3::ZERO);
		assert_eq!(super::preview_origin("combined-explosion"), bevy::prelude::Vec3::ZERO);
	}
}
