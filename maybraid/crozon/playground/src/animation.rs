use std::f32::consts::PI;

use bevy::prelude::*;
use clap::ValueEnum;

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const WORLD_FORWARD: Vec3 = Vec3::NEG_Z;
const WORLD_LATERAL: Vec3 = Vec3::X;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum AnimationMode {
	#[default]
	Wave,
	Run,
}

#[derive(Component)]
pub struct LimbAnimator {
	pub bone: &'static str,
	pub rest: Quat,
	/// World-space axis that produces forward/back motion for this bone.
	pub world_axis: Vec3,
}

const ANIMATED_BONES: &[&str] = &[
	"humerus.L",
	"forearm.L",
	"humerus.R",
	"forearm.R",
	"femur.L",
	"shin.L",
	"femur.R",
	"shin.R",
];

pub fn init_limb_animators(
	mut commands: Commands,
	rig_roots: Query<&BoneMap, With<CharacterRig>>,
	transforms: Query<&Transform>,
	children_q: Query<&Children>,
	parents_q: Query<&ChildOf>,
	animated: Query<Entity, With<LimbAnimator>>,
) {
	if !animated.is_empty() {
		return;
	}

	let Ok(bone_map) = rig_roots.single() else {
		return;
	};

	if bone_map.by_name.is_empty() {
		return;
	}

	for &bone in ANIMATED_BONES {
		let Some(&entity) = bone_map.by_name.get(bone) else {
			continue;
		};
		let Ok(transform) = transforms.get(entity) else {
			continue;
		};

		let world_rot = world_rotation(entity, &transforms, &parents_q);
		let bone_dir = bone_world_direction(entity, world_rot, &children_q, &transforms);
		let world_axis = sagittal_world_axis(bone_dir);

		commands.entity(entity).insert(LimbAnimator {
			bone,
			rest: transform.rotation,
			world_axis,
		});
	}
}

pub fn animate_limbs(
	config: Res<CharacterConfig>,
	mut limbs: Query<(Entity, &mut Transform, &LimbAnimator)>,
	globals: Query<&GlobalTransform>,
	parents: Query<&ChildOf>,
	time: Res<Time>,
) {
	let t = time.elapsed_secs();

	for (entity, mut transform, animator) in &mut limbs {
		let angle = match config.animation {
			AnimationMode::Wave => wave_angle(animator.bone, t),
			AnimationMode::Run => run_angle(animator.bone, t),
		};

		let parent_rot = parents
			.get(entity)
			.ok()
			.and_then(|child_of| globals.get(child_of.parent()).ok())
			.map(|global| global.rotation())
			.unwrap_or(Quat::IDENTITY);

		transform.rotation =
			world_axis_rotation(animator.rest, parent_rot, animator.world_axis, angle);
	}
}

fn wave_angle(bone: &str, t: f32) -> f32 {
	let s = (t * 0.75).sin();
	match bone {
		"humerus.L" | "humerus.R" => s * 0.65,
		"forearm.L" | "forearm.R" => 0.25 + s * 0.25,
		"femur.L" | "femur.R" => s * 0.35,
		"shin.L" | "shin.R" => s * 0.2,
		_ => 0.0,
	}
}

fn run_angle(bone: &str, t: f32) -> f32 {
	let phase = (t * 2.8).fract();
	match bone {
		"femur.R" => thigh_swing(phase) * 1.15,
		"femur.L" => thigh_swing(phase + 0.5) * 1.15,
		"shin.R" => knee_flex(phase),
		"shin.L" => knee_flex(phase + 0.5),
		"humerus.L" => arm_swing(phase) * 0.85,
		"humerus.R" => arm_swing(phase + 0.5) * 0.85,
		"forearm.L" => 0.55 + arm_swing(phase).max(0.0) * 0.25,
		"forearm.R" => 0.55 + arm_swing(phase + 0.5).max(0.0) * 0.25,
		_ => 0.0,
	}
}

fn thigh_swing(phase: f32) -> f32 {
	let p = phase.fract();
	if p < 0.5 {
		4.0 * p - 1.0
	} else {
		3.0 - 4.0 * p
	}
}

fn arm_swing(phase: f32) -> f32 {
	thigh_swing(phase) * 0.75
}

fn knee_flex(leg_phase: f32) -> f32 {
	let p = leg_phase.fract();
	if p > 0.12 && p < 0.42 {
		let x = (p - 0.12) / 0.3;
		(x * PI).sin() * 1.35
	} else if p < 0.08 {
		0.12
	} else {
		0.05
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

fn world_axis_rotation(rest: Quat, parent_rot: Quat, world_axis: Vec3, angle: f32) -> Quat {
	let rest_global = parent_rot * rest;
	let animated_global = Quat::from_axis_angle(world_axis, angle) * rest_global;
	parent_rot.inverse() * animated_global
}
