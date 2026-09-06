//! Re-export pitch math from `crozon-character-motion`, plus girdle prepare.

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use crozon_character_motion::markers::ApplyTerrainPitch;
use crozon_character_motion::{BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};

use crate::member::{CharacterMembers, CharacterRoot};

pub use crozon_character_motion::pitch::{
	default_half_span, default_half_width, facing_with_tilt, girdle_midpoint,
	half_span_from_girdles, half_width_from_sides, measured_support_half, observed_pitch,
	observed_roll, pitch_weight, roll_weight, step_toward, support_lift, TerrainPitch, MAX_TILT,
	QUADRUPED_FRONT, QUADRUPED_HIND, QUADRUPED_LEFT, QUADRUPED_RIGHT, TILT_RATE,
};

/// Insert [`TerrainPitch`] on character roots that opted in with [`ApplyTerrainPitch`],
/// and refresh wheelbase from live girdles once the pose is in world space.
pub fn prepare_character_terrain_pitch(
	mut commands: Commands,
	new_visuals: Query<
		(Entity, &CharacterMembers),
		(With<CharacterRoot>, With<ApplyTerrainPitch>, Without<TerrainPitch>),
	>,
	mut pitched: Query<
		(&CharacterMembers, &mut TerrainPitch),
		(With<CharacterRoot>, With<ApplyTerrainPitch>),
	>,
	rigs: Query<(&CharacterRig, &BoneMap)>,
	globals: Query<&GlobalTransform>,
) {
	for (entity, members) in &new_visuals {
		let Some(kind) = body_skeleton(members, &rigs) else {
			continue;
		};
		let (front, hind, left, right) = supports(kind, members, &rigs, &globals);
		commands.entity(entity).insert(TerrainPitch::new(
			kind,
			half_span_from_girdles(kind, front, hind),
			half_width_from_sides(kind, left, right),
		));
	}

	for (members, mut pitch) in &mut pitched {
		let Some(kind) = body_skeleton(members, &rigs) else {
			continue;
		};
		let (front, hind, left, right) = supports(kind, members, &rigs, &globals);
		if let Some(span) = measured_support_half(front, hind) {
			pitch.half_span = span;
		}
		if let Some(width) = measured_support_half(left, right) {
			pitch.half_width = width;
		}
	}
}

fn body_skeleton(
	members: &CharacterMembers,
	rigs: &Query<(&CharacterRig, &BoneMap)>,
) -> Option<RigSkeletonKind> {
	for member in members.iter() {
		let Ok((rig, _)) = rigs.get(member) else {
			continue;
		};
		if rig.role == CharacterRigRole::Body {
			return Some(rig.skeleton);
		}
	}
	None
}

fn supports(
	kind: RigSkeletonKind,
	members: &CharacterMembers,
	rigs: &Query<(&CharacterRig, &BoneMap)>,
	globals: &Query<&GlobalTransform>,
) -> (Option<Vec3>, Option<Vec3>, Option<Vec3>, Option<Vec3>) {
	if kind != RigSkeletonKind::Quadruped {
		return (None, None, None, None);
	}
	for member in members.iter() {
		let Ok((rig, bones)) = rigs.get(member) else {
			continue;
		};
		if rig.role != CharacterRigRole::Body {
			continue;
		}
		return (
			girdle_midpoint(named_world(bones, QUADRUPED_FRONT, globals)),
			girdle_midpoint(named_world(bones, QUADRUPED_HIND, globals)),
			girdle_midpoint(named_world(bones, QUADRUPED_LEFT, globals)),
			girdle_midpoint(named_world(bones, QUADRUPED_RIGHT, globals)),
		);
	}
	(None, None, None, None)
}

fn named_world(bones: &BoneMap, names: &[&str], globals: &Query<&GlobalTransform>) -> Vec<Vec3> {
	names
		.iter()
		.filter_map(|name| {
			let entity = bones.by_name.get(*name)?;
			globals.get(*entity).ok().map(|g| g.translation())
		})
		.collect()
}
