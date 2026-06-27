use std::collections::HashMap;

use bevy::prelude::*;
use clap::ValueEnum;
use crozon_rigs::{
	debug::{format_rigged_axis, log_bind_pose, log_pose_deltas, RigPoseDebug},
	humanoid::HumanoidRig,
	rigs::humanoid_v0::HumanoidV0Rig,
	BonePose, Name as RigName,
};
use malo_animations::{
	animations::{Run, Squat, TwoFootedJump, DEFAULT_GRAVITY},
	Animation, Effects,
};

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const RUN_CYCLE_SPEED: f32 = 0.5;
const SQUAT_CYCLE_SPEED: f32 = 0.25;
const JUMP_HEIGHT: f32 = 1.5;

const DEBUG_BONES: &[&str] = &[
	"root",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
	"pelvis.L",
	"pelvis.R",
	"femur.L",
	"femur.R",
	"shin.L",
	"shin.R",
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum AnimationMode {
	#[default]
	Run,
	Squat,
	Jump,
}

#[derive(Resource)]
pub struct AnimationArticulationDebug(RigPoseDebug);

impl Default for AnimationArticulationDebug {
	fn default() -> Self {
		Self(RigPoseDebug::default())
	}
}

#[derive(Component)]
pub struct LimbAnimator {
	pub bone: RigName,
	pub rest: Transform,
}

pub fn init_limb_animators(
	mut commands: Commands,
	config: Res<CharacterConfig>,
	debug: Res<AnimationArticulationDebug>,
	rig_roots: Query<(Entity, &BoneMap), With<CharacterRig>>,
	transforms: Query<&Transform>,
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

		commands.entity(entity).insert(LimbAnimator { bone, rest: *transform });
	}

	if debug.0.enabled {
		let bind_log: Vec<_> = humanoid
			.animation_bones()
			.into_iter()
			.filter(|bone| DEBUG_BONES.contains(&bone.as_str()))
			.filter_map(|bone| {
				let entity = bone_map.by_name.get(bone.as_str())?;
				let transform = transforms.get(*entity).ok()?;
				let axis = format_rigged_axis(humanoid.rigged_axis(&bone));
				Some((bone, *transform, axis))
			})
			.collect();

		log_bind_pose(
			&format!("local rest from glTF animation={:?}", config.animation),
			bind_log.iter().map(|(name, transform, axis)| (name, transform, axis.as_str())),
		);
	}

	commands.entity(rig_entity).insert(humanoid);
}

pub fn animate_limbs(
	config: Res<CharacterConfig>,
	mut debug: ResMut<AnimationArticulationDebug>,
	mut rig: Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	mut armature: Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	mut limbs: Query<(&mut Transform, &LimbAnimator)>,
	time: Res<Time>,
) {
	let t = time.elapsed_secs();
	match config.animation {
		AnimationMode::Run => animate_run(&config, &mut rig, &mut armature, &mut limbs, t),
		AnimationMode::Squat => {
			animate_squat(&config, &mut debug, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::Jump => animate_jump(&config, &mut rig, &mut armature, &mut limbs, t),
	}
}

fn animate_run(
	config: &CharacterConfig,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs);
	let effects = Run::<HumanoidV0Rig>::from_time(t, RUN_CYCLE_SPEED).apply(&mut rig);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_squat(
	config: &CharacterConfig,
	debug: &mut AnimationArticulationDebug,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs);
	let squat_half_speed = 2.0 * SQUAT_CYCLE_SPEED;
	let squat = Squat::<HumanoidV0Rig>::from_time(t, squat_half_speed, squat_half_speed);
	let effects = squat.apply(&mut rig);
	apply_effects(config.transform, effects, armature);

	if debug.0.should_log(t) {
		let phase = squat.cycle_phase();
		let lengths = rig.segment_lengths();
		let drop = squat.vertical_drop(lengths);
		let rest_by_bone = rest_transforms(limbs);
		let move_label = effects
			.r#move
			.map(|t| crozon_rigs::debug::format_vec3(t.translation))
			.unwrap_or_else(|| "none".into());
		let header = vec![
			format!("t={t:.2}s phase={phase:.3}"),
			format!(
				"envelope: depth={:.3} femur_swing={:.3} shin_flex={:.3} root_swing={:.3} vertical_drop={:.4}",
				squat.depth(),
				squat.femur_swing(),
				squat.shin_flex(),
				squat.root_swing(),
				drop,
			),
			format!(
				"segment_lengths: femur={:.4} shin={:.4} effects.move={move_label}",
				lengths.femur, lengths.shin
			),
		];

		log_pose_deltas(
			"squat articulation debug",
			rig.pose(),
			&rest_by_bone,
			DEBUG_BONES,
			|name| format_rigged_axis(rig.rigged_axis(name)),
			&header,
		);
	}

	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_jump(
	config: &CharacterConfig,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs);
	let lengths = rig.segment_lengths();
	let jump = TwoFootedJump::<HumanoidV0Rig>::auto_scale(t, DEFAULT_GRAVITY, JUMP_HEIGHT, lengths)
		.with_slower_initial_squat_down_by(0.75);
	let effects = jump.apply(&mut rig);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn apply_effects(
	bind: Transform,
	effects: Effects,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
) {
	let Ok(mut transform) = armature.single_mut() else {
		return;
	};

	*transform = bind;
	if let Some(offset) = effects.r#move {
		transform.translation += offset.translation;
		transform.rotation = offset.rotation * transform.rotation;
		transform.scale *= offset.scale;
	}
}

fn rest_transforms(limbs: &Query<(&mut Transform, &LimbAnimator)>) -> HashMap<RigName, Transform> {
	limbs
		.iter()
		.map(|(_, animator)| (animator.bone.clone(), animator.rest))
		.collect()
}

fn marshal_limbs_into_pose(
	rig: &mut HumanoidV0Rig,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
) {
	for (_, animator) in limbs.iter_mut() {
		rig.pose.insert(BonePose::new(animator.bone.clone(), animator.rest));
	}
}

fn marshal_pose_to_limbs(rig: &HumanoidV0Rig, limbs: &mut Query<(&mut Transform, &LimbAnimator)>) {
	for (mut transform, animator) in limbs.iter_mut() {
		if let Some(pose) = rig.pose.get(&animator.bone) {
			*transform = pose.transform;
		}
	}
}
