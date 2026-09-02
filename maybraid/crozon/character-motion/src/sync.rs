//! Keep host motion markers in sync with the shown (or desired) LOD level.
//!
//! Runtime truth is on the **host**: [`AnimateBones`] / [`AnimateEffects`] on body
//! rigs, [`ApplyTerrainPitch`] on character roots. Expensive systems filter with
//! `With<…>`. This system inserts/removes those markers from [`motion_policy`].

use bevy::ecs::query::Has;
use bevy::prelude::*;
use lod::{LodLevelRoot, LodLevelRoots, LodSceneHost, LodSceneLevel};

use crate::clip::AnimRefRoot;
use crate::markers::{AnimateBones, AnimateEffects, ApplyTerrainPitch};
use crate::pitch::TerrainPitch;
use crate::policy::motion_policy;
use crate::rig::{CharacterRig, CharacterRigRole};
use crate::shown::shown_level_root;

/// Insert/remove host motion markers from the shown band (else desired / High).
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
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	visibilities: Query<&Visibility>,
	desired: Query<&LodSceneLevel, With<LodSceneHost>>,
) {
	for (entity, rig, has_bones, has_effects) in &bodies {
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		let policy = motion_policy(motion_level(
			entity,
			&children,
			&level_roots_bags,
			&root_keys,
			&visibilities,
			&desired,
		));
		set_marker::<AnimateBones>(&mut commands, entity, policy.bones, has_bones);
		set_marker::<AnimateEffects>(&mut commands, entity, policy.effects, has_effects);
	}

	for (entity, has_pitch) in &pitch_hosts {
		let policy = motion_policy(motion_level(
			entity,
			&children,
			&level_roots_bags,
			&root_keys,
			&visibilities,
			&desired,
		));
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
