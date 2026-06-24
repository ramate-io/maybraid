use std::f32::consts::PI;

use bevy::prelude::*;
use clap::ValueEnum;

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const WORLD_FORWARD: Vec3 = Vec3::NEG_Z;
const WORLD_LATERAL: Vec3 = Vec3::X;

const RUN_CYCLE_SPEED: f32 = 0.5;

/// Base pitch from T-pose toward a natural running arm carriage (radians).
const RUN_ARM_DOWN: f32 = 0.85;
/// Base elbow flex while running (radians).
const RUN_ELBOW_BEND: f32 = 1.25;
/// Extra elbow flex while pumping through a stride.
const RUN_ELBOW_PUMP: f32 = 0.5;
/// Shoulder counter-swing amplitude.
const RUN_SHOULDER_SWING: f32 = 0.14;
/// Shoulder rise/drop with arm pump.
const RUN_SHOULDER_LIFT: f32 = 0.07;
/// Hip counter-swing amplitude.
const RUN_HIP_SWING: f32 = 0.1;
/// Hip rise/drop with leg swing.
const RUN_HIP_LIFT: f32 = 0.06;

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
	/// World-space axis for forward/back swing.
	pub swing_axis: Vec3,
	/// World-space axis for pitch (arm down) or hinge flex (elbow/knee).
	pub flex_axis: Vec3,
	/// +1 or −1 so mirrored bones flex the same way visually.
	pub flex_sign: f32,
}

const ANIMATED_BONES: &[&str] = &[
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"forearm.L",
	"humerus.R",
	"forearm.R",
	"pelvis.L",
	"pelvis.R",
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
		let (swing_axis, flex_axis) = bone_axes(bone, bone_dir);
		let flex_sign = if bone.starts_with("forearm.") {
			forward_flex_sign(bone_dir, flex_axis)
		} else {
			1.0
		};

		commands.entity(entity).insert(LimbAnimator {
			bone,
			rest: transform.rotation,
			swing_axis,
			flex_axis,
			flex_sign,
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
		let parent_rot = parents
			.get(entity)
			.ok()
			.and_then(|child_of| globals.get(child_of.parent()).ok())
			.map(|global| global.rotation())
			.unwrap_or(Quat::IDENTITY);

		transform.rotation = match config.animation {
			AnimationMode::Wave => {
				let swing = wave_angle(animator.bone, t);
				compose_world_rotations(
					animator.rest,
					parent_rot,
					animator.swing_axis,
					swing,
					animator.flex_axis,
					wave_flex_angle(animator.bone, t) * animator.flex_sign,
				)
			}
			AnimationMode::Run => {
				let (swing, flex) = run_pose(animator.bone, t, animator.flex_sign);
				compose_world_rotations(
					animator.rest,
					parent_rot,
					animator.swing_axis,
					swing,
					animator.flex_axis,
					flex,
				)
			}
		};
	}
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

fn wave_angle(bone: &str, t: f32) -> f32 {
	let s = (t * 0.75).sin();
	match bone {
		"humerus.L" | "humerus.R" => s * 0.65,
		"femur.L" | "femur.R" => s * 0.35,
		_ => 0.0,
	}
}

fn wave_flex_angle(bone: &str, t: f32) -> f32 {
	let s = (t * 0.75).sin();
	match bone {
		"forearm.L" | "forearm.R" => 0.25 + s * 0.25,
		"shin.L" | "shin.R" => s * 0.2,
		_ => 0.0,
	}
}

fn run_pose(bone: &str, t: f32, flex_sign: f32) -> (f32, f32) {
	let phase = (t * RUN_CYCLE_SPEED).fract();

	let leg = |offset: f32| thigh_swing(phase + offset);
	let left_swing = -arm_swing(phase + 0.5);
	let right_swing = arm_swing(phase);

	match bone {
		"shoulder.L" => {
			(left_swing * RUN_SHOULDER_SWING, -shoulder_lift(left_swing, RUN_SHOULDER_LIFT))
		}
		"shoulder.R" => {
			(right_swing * RUN_SHOULDER_SWING, -shoulder_lift(right_swing, RUN_SHOULDER_LIFT))
		}
		"humerus.L" => (left_swing * 0.75, -RUN_ARM_DOWN),
		"humerus.R" => (right_swing * 0.75, RUN_ARM_DOWN),
		"forearm.L" => (0.0, elbow_flex(left_swing, phase, flex_sign)),
		"forearm.R" => (0.0, elbow_flex(right_swing, phase, flex_sign)),
		"pelvis.L" => (leg(0.5) * RUN_HIP_SWING, -hip_lift(leg(0.5), RUN_HIP_LIFT)),
		"pelvis.R" => (leg(0.0) * RUN_HIP_SWING, hip_lift(leg(0.0), RUN_HIP_LIFT)),
		"femur.L" => (leg(0.5) * 1.05, 0.0),
		"femur.R" => (leg(0.0) * 1.05, 0.0),
		"shin.L" => (0.0, knee_flex(phase + 0.5)),
		"shin.R" => (0.0, knee_flex(phase)),
		_ => (0.0, 0.0),
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

fn elbow_flex(arm_swing: f32, phase: f32, flex_sign: f32) -> f32 {
	let pump = arm_swing.abs();
	let cycle = ((phase + arm_swing.signum() * 0.125) * PI * 4.0).sin().abs();
	flex_sign * (RUN_ELBOW_BEND + pump * RUN_ELBOW_PUMP + cycle * 0.35)
}

fn shoulder_lift(arm_swing: f32, amplitude: f32) -> f32 {
	arm_swing * amplitude
}

fn hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

/// ~90° knee joint angle at front and rear of the stride.
const KNEE_NEUTRAL: f32 = PI * 0.5;
/// Contracted (< 90°): tucked knee on forward swing (rear → front).
const KNEE_CONTRACTED: f32 = 2.15;
/// Extended (> 90°): straighter knee on backswing (front → rear).
const KNEE_EXTENDED: f32 = 0.35;

fn knee_flex(leg_phase: f32) -> f32 {
	let p = leg_phase.fract();
	let peak = if p < 0.5 { KNEE_EXTENDED } else { KNEE_CONTRACTED };
	let t = if p < 0.5 { p * 2.0 } else { (p - 0.5) * 2.0 };
	KNEE_NEUTRAL + (t * PI).sin() * (peak - KNEE_NEUTRAL)
}

fn forward_flex_sign(bone_dir: Vec3, axis: Vec3) -> f32 {
	const TEST: f32 = 0.12;
	let neg = Quat::from_axis_angle(axis, -TEST) * bone_dir;
	let pos = Quat::from_axis_angle(axis, TEST) * bone_dir;
	let neg_forward = (neg - bone_dir).dot(WORLD_FORWARD);
	let pos_forward = (pos - bone_dir).dot(WORLD_FORWARD);
	if neg_forward < pos_forward {
		-1.0
	} else {
		1.0
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
