//! After a brief ragdoll settle, hide the corpse mesh and play spirit wisps.

use bevy::prelude::*;
use character_ragdoll::{Corpse, RagdollState};
use characters::CharacterRoot;
use damage::{DespawnAfter, Downed};
use maybraid_vfx::{
	ExplosionPalette, EffectRecipe, SpawnVfxExt, VfxLibrary, VfxSpawn,
};

/// Corpse visual is dissolving; gameplay body gets a matching despawn clock.
#[derive(Component, Clone, Copy, Debug)]
pub struct SpiritDissolveStarted {
	pub at: f32,
}

/// How long wisps rise after the body is hidden.
pub const SPIRIT_DISSOLVE_SECS: f32 = 2.85;

/// Wait for ragdoll motion to settle (or this fallback without a solver).
pub const CORPSE_SETTLE_SECS: f32 = 0.85;

#[derive(Component, Clone, Debug)]
pub(crate) struct CorpseSettleClock(Timer);

pub(crate) fn tick_corpse_settle(
	mut commands: Commands,
	corpses: Query<
		Entity,
		(
			With<Corpse>,
			With<CharacterRoot>,
			Without<SpiritDissolveStarted>,
			Without<CorpseSettleClock>,
		),
	>,
) {
	for entity in &corpses {
		commands.entity(entity).insert(CorpseSettleClock(Timer::from_seconds(
			CORPSE_SETTLE_SECS,
			TimerMode::Once,
		)));
	}
}

pub(crate) fn begin_spirit_dissolve(
	time: Res<Time>,
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
	mut settle: Query<(Entity, &mut CorpseSettleClock, &GlobalTransform), Without<SpiritDissolveStarted>>,
	ragdolls: Query<&RagdollState>,
	parents: Query<&ChildOf>,
	hosts: Query<(Entity, &RagdollState)>,
) {
	let Some(library) = library else {
		return;
	};
	let now = time.elapsed_secs();
	for (visual, mut clock, transform) in &mut settle {
		clock.0.tick(time.delta());
		let ragdoll_settled = ragdolls
			.iter()
			.any(|state| state.corpse() == visual && state.is_sleeping());
		if !ragdoll_settled && !clock.0.is_finished() {
			continue;
		}
		if let Ok(mut entity) = commands.get_entity(visual) {
			entity.try_insert((
				SpiritDissolveStarted { at: now },
				Visibility::Hidden,
				DespawnAfter::seconds(SPIRIT_DISSOLVE_SECS),
			));
		}
		for (host, state) in &hosts {
			if state.corpse() == visual {
				commands.entity(host).remove::<RagdollState>();
			}
		}
		commands.spawn_vfx(
			&library.spirit_dissolve.effect_definition(),
			VfxSpawn {
				transform: Transform::from_translation(transform.translation()),
				scale: 1.0,
				intensity: 1.0,
				playback: 1.0,
				palette: Some(ExplosionPalette::celestial()),
				seed: Some(visual.to_bits() ^ 0x5a17_d15c),
				..default()
			},
		);
		if let Ok(parent) = parents.get(visual) {
			if let Ok(mut body) = commands.get_entity(parent.parent()) {
				body.try_insert(DespawnAfter::seconds(SPIRIT_DISSOLVE_SECS));
			}
		}
		commands.entity(visual).remove::<ChildOf>();
		commands.entity(visual).remove::<CorpseSettleClock>();
	}
}

/// Drain detached corpse visuals that finished their spirit clock.
pub(crate) fn tick_detached_corpse_despawns(
	time: Res<Time>,
	mut commands: Commands,
	mut queued: Query<(Entity, &mut DespawnAfter), (With<SpiritDissolveStarted>, With<Corpse>)>,
) {
	for (entity, mut despawn) in &mut queued {
		despawn.tick_secs(time.delta_secs());
		if despawn.is_finished() {
			commands.entity(entity).try_despawn();
		}
	}
}

/// When a retired body expires, remove any lingering corpse visuals still parented to it.
pub(crate) fn despawn_corpse_bodies(
	time: Res<Time>,
	mut commands: Commands,
	mut bodies: Query<(Entity, &mut DespawnAfter), (With<Downed>, Without<Corpse>)>,
	children: Query<(Entity, &ChildOf), With<Corpse>>,
) {
	for (body, mut despawn) in &mut bodies {
		despawn.tick_secs(time.delta_secs());
		if !despawn.is_finished() {
			continue;
		}
		for (visual, child_of) in &children {
			if child_of.parent() == body {
				commands.entity(visual).try_despawn();
			}
		}
		if let Ok(mut entity) = commands.get_entity(body) {
			entity.try_despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use player::Npc;

	#[test]
	fn spirit_dissolve_despawns_visual_after_delay() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_systems(Update, (tick_corpse_settle, tick_detached_corpse_despawns).chain());
		let visual = app
			.world_mut()
			.spawn((
				Corpse {
					source: None,
					point: Vec3::ZERO,
					inherited_velocity: Vec3::ZERO,
				},
				CharacterRoot,
				Visibility::Visible,
				GlobalTransform::IDENTITY,
			))
			.id();
		app.update();
		assert!(app.world().get::<CorpseSettleClock>(visual).is_some());

		app.world_mut().entity_mut(visual).insert((
			SpiritDissolveStarted { at: 0.0 },
			DespawnAfter::seconds(0.0),
		));
		app.update();
		assert!(!app.world().entities().contains(visual));
	}

	#[test]
	fn body_despawn_clears_parented_corpse_visual() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Last, despawn_corpse_bodies);
		let body = app
			.world_mut()
			.spawn((
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
				Npc,
				DespawnAfter::seconds(0.0),
			))
			.id();
		let visual = app
			.world_mut()
			.spawn((Corpse {
				source: None,
				point: Vec3::ZERO,
				inherited_velocity: Vec3::ZERO,
			}, ChildOf(body)))
			.id();
		app.update();
		assert!(!app.world().entities().contains(body));
		assert!(!app.world().entities().contains(visual));
	}
}
