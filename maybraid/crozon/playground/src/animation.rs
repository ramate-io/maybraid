use bevy::prelude::*;
use clap::ValueEnum;
use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Name as RigName};
use malo_animations::{
	animations::{Run, Squat},
	Animation,
};

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const WORLD_FORWARD: Vec3 = Vec3::NEG_Z;
const WORLD_LATERAL: Vec3 = Vec3::X;

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
	pub rest: Quat,
	pub rest_translation: Vec3,
	/// World-space axis for forward/back swing.
	pub swing_axis: Vec3,
	/// World-space axis for pitch (arm down) or hinge flex (elbow/knee).
	pub flex_axis: Vec3,
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
	}

	let humanoid = HumanoidV0Rig::imported();
	for bone in humanoid.animation_bones() {
		let Some(&entity) = bone_map.by_name.get(bone.as_str()) else {
			continue;
		};
		let Ok(transform) = transforms.get(entity) else {
			continue;
		};

		let world_rot = world_rotation(entity, &transforms, &parents_q);
		let bone_dir = bone_world_direction(entity, world_rot, &children_q, &transforms);
		let (swing_axis, flex_axis) = bone_axes(bone.as_str(), bone_dir);

		commands.entity(entity).insert(LimbAnimator {
			bone,
			rest: transform.rotation,
			rest_translation: transform.translation,
			swing_axis,
			flex_axis,
		});
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

	Run::<HumanoidV0Rig>::from_time(t, RUN_CYCLE_SPEED).apply(&mut rig);
	apply_rig_pose_to_limbs(&rig, limbs, globals, parents);
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

	Squat::<HumanoidV0Rig>::from_time(t, SQUAT_CYCLE_SPEED).apply(&mut rig);
	apply_rig_pose_to_limbs(&rig, limbs, globals, parents);
}

fn apply_rig_pose_to_limbs(
	rig: &HumanoidV0Rig,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
	globals: &Query<&GlobalTransform>,
	parents: &Query<&ChildOf>,
) {
	for (entity, mut transform, animator) in limbs.iter_mut() {
		let Some(articulation) = rig.pose.get(&animator.bone) else {
			continue;
		};
		let parent_global = parent_transform(entity, globals, parents);
		let parent_rot = parent_global.rotation();
		transform.rotation = compose_world_rotations(
			animator.rest,
			parent_rot,
			animator.swing_axis,
			articulation.swing,
			animator.flex_axis,
			articulation.flex,
		);
		transform.translation = compose_world_translation(
			animator.rest_translation,
			parent_global,
			articulation.transform.translation,
		);
	}
}

fn parent_transform(
	entity: Entity,
	globals: &Query<&GlobalTransform>,
	parents: &Query<&ChildOf>,
) -> GlobalTransform {
	parents
		.get(entity)
		.ok()
		.and_then(|child_of| globals.get(child_of.parent()).ok())
		.copied()
		.unwrap_or(GlobalTransform::IDENTITY)
}

fn bone_axes(bone: &str, bone_dir: Vec3) -> (Vec3, Vec3) {
	let sagittal = sagittal_world_axis(bone_dir);
	match bone {
		"forearm.L" | "forearm.R" | "shin.L" | "shin.R" => (sagittal, sagittal),
		"humerus.L" | "humerus.R" => (sagittal, pitch_down_axis(bone_dir)),
		"shoulder.L" | "shoulder.R" | "pelvis.L" | "pelvis.R" => {
			(sagittal, vertical_lift_axis(bone_dir))
		}
		_ => (sagittal, hinge_axis(bone_dir, sagittal)),
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

fn bone_world_direction(
	entity: Entity,
	world_rot: Quat,
	children_q: &Query<&Children>,
	transforms: &Query<&Transform>,
) -> Vec3 {
	if let Ok(children) = children_q.get(entity) {
		for child in children.iter() {
			if let Ok(child_transform) = transforms.get(child) {
				let local = Vec3::from(child_transform.translation);
				if local.length_squared() > f32::EPSILON {
					return (world_rot * local).normalize();
				}
			}
		}
	}

	(world_rot * Vec3::Y).normalize_or(Vec3::NEG_Y)
}

/// Pick the world axis whose small rotation moves the bone forward/back, not side-to-side.
fn sagittal_world_axis(bone_dir: Vec3) -> Vec3 {
	const CANDIDATES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
	const TEST_ANGLE: f32 = 0.1;

	let mut best_axis = Vec3::X;
	let mut best_score = f32::NEG_INFINITY;

	for axis in CANDIDATES {
		if axis.cross(bone_dir).length_squared() < f32::EPSILON {
			continue;
		}

		let rotated = Quat::from_axis_angle(axis, TEST_ANGLE) * bone_dir;
		let delta = rotated - bone_dir;
		let forward = delta.dot(WORLD_FORWARD).abs();
		let lateral = delta.dot(WORLD_LATERAL).abs();
		let score = forward / (lateral + 1e-3);

		if score > best_score {
			best_score = score;
			best_axis = axis;
		}
	}

	best_axis
}

/// Axis that lifts or drops the bone mostly along world up.
fn vertical_lift_axis(bone_dir: Vec3) -> Vec3 {
	const CANDIDATES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
	const TEST_ANGLE: f32 = 0.1;

	let mut best_axis = Vec3::Z;
	let mut best_lift = 0.0_f32;

	for axis in CANDIDATES {
		if axis.cross(bone_dir).length_squared() < f32::EPSILON {
			continue;
		}

		let rotated = Quat::from_axis_angle(axis, TEST_ANGLE) * bone_dir;
		let lift = (rotated.y - bone_dir.y).abs();
		if lift > best_lift {
			best_lift = lift;
			best_axis = axis;
		}
	}

	best_axis
}

/// Axis that pitches a T-pose arm toward world down.
fn pitch_down_axis(bone_dir: Vec3) -> Vec3 {
	const CANDIDATES: [Vec3; 3] = [Vec3::X, Vec3::Y, Vec3::Z];
	const TEST_ANGLE: f32 = 0.1;

	let mut best_axis = Vec3::Z;
	let mut best_down = f32::NEG_INFINITY;

	for axis in CANDIDATES {
		if axis.cross(bone_dir).length_squared() < f32::EPSILON {
			continue;
		}

		let rotated = Quat::from_axis_angle(axis, TEST_ANGLE) * bone_dir;
		let downward = rotated.y - bone_dir.y;
		if downward < best_down {
			best_down = downward;
			best_axis = axis;
		}
	}

	best_axis
}

fn hinge_axis(bone_dir: Vec3, swing_axis: Vec3) -> Vec3 {
	bone_dir.cross(swing_axis).normalize_or(Vec3::Y)
}

fn compose_world_rotations(
	rest: Quat,
	parent_rot: Quat,
	swing_axis: Vec3,
	swing: f32,
	flex_axis: Vec3,
	flex: f32,
) -> Quat {
	let mut global = parent_rot * rest;
	if flex.abs() > f32::EPSILON {
		global = Quat::from_axis_angle(flex_axis, flex) * global;
	}
	if swing.abs() > f32::EPSILON {
		global = Quat::from_axis_angle(swing_axis, swing) * global;
	}
	parent_rot.inverse() * global
}

fn compose_world_translation(
	rest_translation: Vec3,
	parent_global: GlobalTransform,
	translation: Vec3,
) -> Vec3 {
	let parent_rot = parent_global.rotation();
	rest_translation + parent_rot.inverse() * translation
}
