use bevy::prelude::*;

use crate::skinning::{BoneMap, CharacterRig};

#[derive(Component)]
pub struct LimbAnimator {
	pub rest: Quat,
	pub axis: Vec3,
	pub amplitude: f32,
	pub speed: f32,
	pub phase: f32,
}

struct LimbSpec {
	bone: &'static str,
	axis: Vec3,
	amplitude: f32,
	speed: f32,
	phase: f32,
}

const LIMB_SPECS: &[LimbSpec] = &[
	LimbSpec { bone: "humerus.L", axis: Vec3::X, amplitude: 0.55, speed: 1.0, phase: 0.0 },
	LimbSpec { bone: "forearm.L", axis: Vec3::X, amplitude: 0.4, speed: 1.0, phase: 0.35 },
	LimbSpec { bone: "humerus.R", axis: Vec3::X, amplitude: 0.55, speed: 1.0, phase: std::f32::consts::PI },
	LimbSpec {
		bone: "forearm.R",
		axis: Vec3::X,
		amplitude: 0.4,
		speed: 1.0,
		phase: std::f32::consts::PI + 0.35,
	},
	LimbSpec { bone: "femur.L", axis: Vec3::X, amplitude: 0.5, speed: 0.85, phase: 0.0 },
	LimbSpec { bone: "shin.L", axis: Vec3::X, amplitude: 0.45, speed: 0.85, phase: 0.45 },
	LimbSpec {
		bone: "femur.R",
		axis: Vec3::X,
		amplitude: 0.5,
		speed: 0.85,
		phase: std::f32::consts::PI,
	},
	LimbSpec {
		bone: "shin.R",
		axis: Vec3::X,
		amplitude: 0.45,
		speed: 0.85,
		phase: std::f32::consts::PI + 0.45,
	},
];

pub fn init_limb_animators(
	mut commands: Commands,
	rig_roots: Query<&BoneMap, With<CharacterRig>>,
	transforms: Query<&Transform>,
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

	for spec in LIMB_SPECS {
		let Some(&entity) = bone_map.by_name.get(spec.bone) else {
			continue;
		};
		let Ok(transform) = transforms.get(entity) else {
			continue;
		};

		commands.entity(entity).insert(LimbAnimator {
			rest: transform.rotation,
			axis: spec.axis,
			amplitude: spec.amplitude,
			speed: spec.speed,
			phase: spec.phase,
		});
	}
}

pub fn animate_limbs(time: Res<Time>, mut limbs: Query<(&mut Transform, &LimbAnimator)>) {
	let t = time.elapsed_secs();

	for (mut transform, animator) in &mut limbs {
		let angle = (t * animator.speed + animator.phase).sin() * animator.amplitude;
		transform.rotation = animator.rest * Quat::from_axis_angle(animator.axis, angle);
	}
}
