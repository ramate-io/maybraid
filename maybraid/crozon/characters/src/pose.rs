//! Realize [`ActiveRigPose`] onto a ready [`BoneMap`] (proportion × bind).
//!
//! Parallel to material-ref: identity lives on the rig member; this system applies
//! it every frame because GLTF spawn can reset bone transforms.

use bevy::prelude::*;

use crate::anim::AnimBone;
use crate::rig::{
	bind_scales_ready, bone_map_ready, ActiveRigPose, BoneMap, CharacterRig, ResolvedPoseApplied,
	RigBindScales,
};

/// Apply [`ActiveRigPose`] layers to named bones. Skips rotation on [`AnimBone`]s
/// so the animation mailbox can own those joints.
pub fn maintain_resolved_pose(
	mut commands: Commands,
	mut rig_roots: Query<
		(
			Entity,
			&BoneMap,
			&ActiveRigPose,
			&mut RigBindScales,
			&CharacterRig,
			Has<ResolvedPoseApplied>,
		),
		With<CharacterRig>,
	>,
	mut transforms: Query<&mut Transform>,
	anim_bones: Query<(), With<AnimBone>>,
) {
	for (entity, bone_map, active_pose, mut bind_scales, rig, pose_applied) in &mut rig_roots {
		if !bone_map_ready(bone_map, rig.skeleton) {
			continue;
		}

		for (bone_name, bone_entity) in &bone_map.by_name {
			if bind_scales.scales.contains_key(bone_name)
				&& bind_scales.translations.contains_key(bone_name)
				&& bind_scales.rotations.contains_key(bone_name)
			{
				continue;
			}
			let Ok(transform) = transforms.get(*bone_entity) else {
				continue;
			};
			bind_scales.scales.entry(bone_name.clone()).or_insert(transform.scale);
			bind_scales
				.translations
				.entry(bone_name.clone())
				.or_insert(transform.translation);
			bind_scales.rotations.entry(bone_name.clone()).or_insert(transform.rotation);
		}

		for (bone_name, bone_entity) in &bone_map.by_name {
			let scale_mult = active_pose.pose.scale_for_bone(bone_name);
			let trans_mult = active_pose.pose.translation_for_bone(bone_name);
			let rot_offset = active_pose.pose.rotation_for_bone(bone_name);
			if scale_mult == Vec3::ONE && trans_mult == Vec3::ONE && rot_offset == Quat::IDENTITY {
				continue;
			}
			let Ok(mut transform) = transforms.get_mut(*bone_entity) else {
				continue;
			};
			if scale_mult != Vec3::ONE {
				if let Some(bind_scale) = bind_scales.scales.get(bone_name) {
					transform.scale = *bind_scale * scale_mult;
				}
			}
			if trans_mult != Vec3::ONE {
				if let Some(bind_trans) = bind_scales.translations.get(bone_name) {
					transform.translation = *bind_trans * trans_mult;
				}
			}
			if rot_offset != Quat::IDENTITY && !anim_bones.contains(*bone_entity) {
				if let Some(bind_rot) = bind_scales.rotations.get(bone_name) {
					transform.rotation = rot_offset * *bind_rot;
				}
			}
		}

		if !pose_applied && bind_scales_ready(&bind_scales, bone_map, rig.skeleton) {
			commands.entity(entity).try_insert(ResolvedPoseApplied);
		}
	}
}
