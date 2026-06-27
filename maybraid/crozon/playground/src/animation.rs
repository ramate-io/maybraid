use bevy::prelude::*;
use clap::ValueEnum;
use crozon_rigs::{
	articulation::{bone_axes, bone_world_direction, forward_flex_sign, BoneArticulationFrame},
	humanoid::HumanoidRig,
	rigs::humanoid_v0::HumanoidV0Rig,
	BonePose, Name as RigName,
};
use malo_animations::{
	animations::{Run, Squat},
	Animation,
};

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const RUN_CYCLE_SPEED: f32 = 0.5;
const SQUAT_CYCLE_SPEED: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum AnimationMode {
	#[default]
	Run,
	Squat,
}

#[derive(Component)]
pub struct LimbAnimator {
	pub bone: RigName,
	pub rest: Transform,
}

pub fn init_limb_animators(
	mut commands: Commands,
	rig_roots: Query<(Entity, &BoneMap), With<CharacterRig>>,
	transforms: Query<&Transform>,
	children_q: Query<&Children>,
	parents_q: Query<&ChildOf>,
	animated: Query<Entity, With<LimbAnimator>>,
) {
	if !animated.is_empty() {
		return;
	}

	let Ok((rig_entity, bone_map)) = rig_roots.single() else {
		return;
	};

	if bone_map.by_name.is_empty() {
		return;
	};

	let mut humanoid = HumanoidV0Rig::imported();
	for bone in humanoid.animation_bones() {
		let Some(&entity) = bone_map.by_name.get(bone.as_str()) else {
			continue;
		};
		let Ok(transform) = transforms.get(entity) else {
			continue;
		};

		let world_rot = world_rotation(entity, &transforms, &parents_q);
		let bone_dir =
			bone_world_direction_from_entity(entity, world_rot, &children_q, &transforms);
		let (swing_axis, flex_axis) = bone_axes(bone.as_str(), bone_dir);
		let flex_sign = if bone.as_str().starts_with("forearm.") {
			forward_flex_sign(bone_dir, flex_axis)
		} else {
			1.0
		};
		humanoid.set_articulation_frame(
			bone.clone(),
			BoneArticulationFrame::new(swing_axis, flex_axis, flex_sign),
		);

		commands.entity(entity).insert(LimbAnimator { bone, rest: *transform });
	}

	commands.entity(rig_entity).insert(humanoid);
}

pub fn animate_limbs(
	config: Res<CharacterConfig>,
	mut rig: Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	mut limbs: Query<(Entity, &mut Transform, &LimbAnimator)>,
	globals: Query<&GlobalTransform>,
	parents: Query<&ChildOf>,
	time: Res<Time>,
) {
	match config.animation {
		AnimationMode::Run => {
			animate_run(&mut rig, &mut limbs, &globals, &parents, time.elapsed_secs())
		}
		AnimationMode::Squat => {
			animate_squat(&mut rig, &mut limbs, &globals, &parents, time.elapsed_secs())
		}
	}
}

fn animate_run(
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
	globals: &Query<&GlobalTransform>,
	parents: &Query<&ChildOf>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, globals, parents);
	Run::<HumanoidV0Rig>::from_time(t, RUN_CYCLE_SPEED).apply(&mut rig);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_squat(
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
	globals: &Query<&GlobalTransform>,
	parents: &Query<&ChildOf>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, globals, parents);
	Squat::<HumanoidV0Rig>::from_time(t, SQUAT_CYCLE_SPEED).apply(&mut rig);
	marshal_pose_to_limbs(&rig, limbs);
}

fn marshal_limbs_into_pose(
	rig: &mut HumanoidV0Rig,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
	globals: &Query<&GlobalTransform>,
	parents: &Query<&ChildOf>,
) {
	rig.clear_parent_rots();
	for (entity, _, animator) in limbs.iter_mut() {
		rig.pose.insert(BonePose::new(animator.bone.clone(), animator.rest));
		let parent_rot = parents
			.get(entity)
			.ok()
			.and_then(|child_of| globals.get(child_of.parent()).ok())
			.map(|global| global.rotation())
			.unwrap_or(Quat::IDENTITY);
		rig.set_parent_rot(animator.bone.clone(), parent_rot);
	}
}

fn marshal_pose_to_limbs(
	rig: &HumanoidV0Rig,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
) {
	for (_, mut transform, animator) in limbs.iter_mut() {
		if let Some(pose) = rig.pose.get(&animator.bone) {
			*transform = pose.transform;
		}
	}
}

fn world_rotation(
	entity: Entity,
	transforms: &Query<&Transform>,
	parents: &Query<&ChildOf>,
) -> Quat {
	let Ok(transform) = transforms.get(entity) else {
		return Quat::IDENTITY;
	};

	let mut rot = transform.rotation;
	let mut current = entity;
	while let Ok(child_of) = parents.get(current) {
		let parent = child_of.parent();
		let Ok(parent_transform) = transforms.get(parent) else {
			break;
		};
		rot = parent_transform.rotation * rot;
		current = parent;
	}
	rot
}

fn bone_world_direction_from_entity(
	entity: Entity,
	world_rot: Quat,
	children_q: &Query<&Children>,
	transforms: &Query<&Transform>,
) -> Vec3 {
	let child_local = children_q.get(entity).ok().and_then(|children| {
		for child in children.iter() {
			if let Ok(child_transform) = transforms.get(child) {
				let local = Vec3::from(child_transform.translation);
				if local.length_squared() > f32::EPSILON {
					return Some(local);
				}
			}
		}
		None
	});

	bone_world_direction(world_rot, child_local)
}
