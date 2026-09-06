//! Re-export pitch math from `crozon-character-motion`, plus girdle prepare.

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use crozon_character_motion::markers::ApplyTerrainPitch;
use crozon_character_motion::{BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};

use crate::member::{CharacterMembers, CharacterRoot};

pub use crozon_character_motion::pitch::{
	adopt_yaw, default_half_span, default_half_width, facing_with_support_tilt, facing_with_tilt,
	girdle_midpoint, half_span_from_girdles, half_width_from_sides, measured_support_half,
	observed_pitch, observed_roll, pitch_weight, pitched_half_run, roll_weight, sagittal_axis,
	sample_facing, smooth_toward, step_toward, support_offset, TerrainPitch, MAX_TILT,
	MIN_SUPPORT_CHANGE, MIN_TILT_CHANGE, QUADRUPED_FRONT, QUADRUPED_HIND, QUADRUPED_LEFT,
	QUADRUPED_RIGHT, SUPPORT_RATE, TILT_RATE, TILT_SMOOTH, YAW_ADOPT,
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
		let mut pitch = TerrainPitch::new(kind, default_half_span(kind), default_half_width(kind));
		if let Some(bones) = body_bones(members, &rigs) {
			record_quadruped_girdles(&mut pitch, kind, bones, &globals);
		}
		commands.entity(entity).insert(pitch);
	}

	for (members, mut pitch) in &mut pitched {
		let Some(kind) = body_skeleton(members, &rigs) else {
			continue;
		};
		if let Some(bones) = body_bones(members, &rigs) {
			record_quadruped_girdles(&mut pitch, kind, bones, &globals);
		}
	}
}

fn record_quadruped_girdles(
	pitch: &mut TerrainPitch,
	kind: RigSkeletonKind,
	bones: &BoneMap,
	globals: &Query<&GlobalTransform>,
) {
	if kind != RigSkeletonKind::Quadruped {
		return;
	}
	pitch.record_girdles(
		named_world(bones, "shoulder.L", globals),
		named_world(bones, "shoulder.R", globals),
		named_world(bones, "hip.L", globals),
		named_world(bones, "hip.R", globals),
	);
}

fn body_skeleton(
	members: &CharacterMembers,
	rigs: &Query<(&CharacterRig, &BoneMap)>,
) -> Option<RigSkeletonKind> {
	body_rig(members, rigs).map(|(rig, _)| rig.skeleton)
}

fn body_bones<'a>(
	members: &CharacterMembers,
	rigs: &'a Query<(&CharacterRig, &BoneMap)>,
) -> Option<&'a BoneMap> {
	body_rig(members, rigs).map(|(_, bones)| bones)
}

fn body_rig<'a>(
	members: &CharacterMembers,
	rigs: &'a Query<(&CharacterRig, &BoneMap)>,
) -> Option<(&'a CharacterRig, &'a BoneMap)> {
	for member in members.iter() {
		let Ok((rig, bones)) = rigs.get(member) else {
			continue;
		};
		if rig.role == CharacterRigRole::Body {
			return Some((rig, bones));
		}
	}
	None
}

fn named_world(bones: &BoneMap, name: &str, globals: &Query<&GlobalTransform>) -> Option<Vec3> {
	let entity = bones.by_name.get(name)?;
	globals.get(*entity).ok().map(|g| g.translation())
}
