//! Keep host motion markers in sync with the shown (or desired) LOD level.
//!
//! Runtime truth is on the **host**: [`AnimateBones`] / [`AnimateEffects`] on body
//! rigs, [`ApplyTerrainPitch`] on character roots. Expensive systems filter with
//! `With<…>`. This system inserts/removes those markers from [`motion_policy`],
//! then clamps mailbox work from the plant's [`IntelligenceLod`].

use bevy::ecs::query::Has;
use bevy::prelude::*;
use intelligence_lod::IntelligenceLod;
use lod::{LodLevelRoot, LodLevelRoots, LodSceneHost, LodSceneLevel};

use crate::clip::AnimRefRoot;
use crate::markers::{AnimateBones, AnimateEffects, ApplyTerrainPitch};
use crate::pitch::TerrainPitch;
use crate::policy::{clamp_intelligence, motion_policy};
use crate::rig::{CharacterRig, CharacterRigRole};
use crate::shown::shown_level_root;

/// Insert/remove host motion markers from the shown band (else desired / High).
///
/// Mid / Far plants drop bones and effects so [`crate::apply_anim_mailbox`]
/// skips them. Missing lod is Near — a local player keeps the mailbox.
pub fn sync_motion_markers(
	mut commands: Commands,
	bodies: Query<
		(Entity, &CharacterRig, Has<AnimateBones>, Has<AnimateEffects>),
		With<AnimRefRoot>,
	>,
	pitch_hosts: Query<
		(Entity, Has<ApplyTerrainPitch>),
		Or<(With<ApplyTerrainPitch>, With<TerrainPitch>)>,
	>,
	children: Query<&Children>,
	child_of: Query<&ChildOf>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	visibilities: Query<&Visibility>,
	desired: Query<&LodSceneLevel, With<LodSceneHost>>,
	lods: Query<&IntelligenceLod>,
) {
	for (entity, rig, has_bones, has_effects) in &bodies {
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		let policy = clamp_intelligence(
			motion_policy(motion_level(
				entity,
				&children,
				&level_roots_bags,
				&root_keys,
				&visibilities,
				&desired,
			)),
			plant_lod(entity, &child_of, &lods),
		);
		set_marker::<AnimateBones>(&mut commands, entity, policy.bones, has_bones);
		set_marker::<AnimateEffects>(&mut commands, entity, policy.effects, has_effects);
	}

	for (entity, has_pitch) in &pitch_hosts {
		let policy = clamp_intelligence(
			motion_policy(motion_level(
				entity,
				&children,
				&level_roots_bags,
				&root_keys,
				&visibilities,
				&desired,
			)),
			plant_lod(entity, &child_of, &lods),
		);
		set_marker::<ApplyTerrainPitch>(&mut commands, entity, policy.pitch, has_pitch);
	}
}

fn motion_level(
	host: Entity,
	children: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	visibilities: &Query<&Visibility>,
	desired: &Query<&LodSceneLevel, With<LodSceneHost>>,
) -> LodSceneLevel {
	if let Some(root) = shown_level_root(host, children, level_roots_bags, root_keys, visibilities)
	{
		if let Ok(key) = root_keys.get(root) {
			return key.0;
		}
	}
	desired.get(host).copied().unwrap_or(LodSceneLevel::High)
}

/// Plant [`IntelligenceLod`], walking toward the root. Band is on the plant, not
/// the nested body. Missing = Near.
fn plant_lod<'a>(
	start: Entity,
	child_of: &Query<&ChildOf>,
	lods: &'a Query<&IntelligenceLod>,
) -> Option<&'a IntelligenceLod> {
	let mut current = Some(start);
	for _ in 0..32 {
		let Some(entity) = current else {
			break;
		};
		if let Ok(lod) = lods.get(entity) {
			return Some(lod);
		}
		current = child_of.get(entity).ok().map(ChildOf::parent);
	}
	None
}

fn set_marker<M: Component + Default>(
	commands: &mut Commands,
	entity: Entity,
	want: bool,
	has: bool,
) {
	match (want, has) {
		(true, false) => {
			commands.entity(entity).insert(M::default());
		}
		(false, true) => {
			commands.entity(entity).remove::<M>();
		}
		_ => {}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::rig::CharacterRig;
	use intelligence_lod::IntelligenceBand;

	fn run_sync(world: &mut World) {
		use bevy::ecs::system::RunSystemOnce;
		world.run_system_once(sync_motion_markers).expect("sync_motion_markers");
	}

	fn body_on(world: &mut World, parent: Entity) -> Entity {
		world
			.spawn((
				CharacterRig { role: CharacterRigRole::Body, ..default() },
				AnimRefRoot::default(),
				AnimateBones,
				AnimateEffects,
				ChildOf(parent),
			))
			.id()
	}

	#[test]
	fn missing_lod_keeps_high_mailbox() {
		let mut world = World::new();
		let plant = world.spawn_empty().id();
		let body = body_on(&mut world, plant);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_some());
		assert!(world.get::<AnimateEffects>(body).is_some());
	}

	#[test]
	fn far_plant_drops_mailbox() {
		let mut world = World::new();
		let plant = world.spawn(IntelligenceLod { band: IntelligenceBand::Far, skips: 0 }).id();
		let visual = world.spawn(ChildOf(plant)).id();
		let body = body_on(&mut world, visual);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_none());
		assert!(world.get::<AnimateEffects>(body).is_none());
	}

	#[test]
	fn near_plant_keeps_mailbox() {
		let mut world = World::new();
		let plant = world.spawn(IntelligenceLod::missing()).id();
		let body = body_on(&mut world, plant);
		run_sync(&mut world);
		assert!(world.get::<AnimateBones>(body).is_some());
		assert!(world.get::<AnimateEffects>(body).is_some());
	}
}
