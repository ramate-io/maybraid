use std::collections::HashMap;

use bevy::prelude::*;
use clap::ValueEnum;
use log::info;
use crozon_rigs::{
	debug::{format_rigged_axis, log_bind_pose, log_pose_deltas, RigPoseDebug},
	humanoid::HumanoidRig,
	rigs::humanoid_v0::HumanoidV0Rig,
	BonePose, Name as RigName,
};
use malo_animations::{
	animations::{FixedTuck, Run, Squat, Tuck, TuckedFlip, TwoFootedJump, TwoFootTuckedFlip, Walk, DEFAULT_GRAVITY, DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED},
	Animation, Effects,
};

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const RUN_CYCLE_SPEED: f32 = 0.5;
const WALK_CYCLE_SPEED: f32 = 0.35;
const SQUAT_CYCLE_SPEED: f32 = 0.25;
const TUCK_CYCLE_SPEED: f32 = 0.6;
const FRONT_FLIP_CYCLE_SPEED: f32 = 0.85;
const JUMP_HEIGHT: f32 = 1.5;
const JUMP_PRE_SQUAT_SPEED: f32 = DEFAULT_PRE_SQUAT_SPEED * 1.2;
const JUMP_LANDING_SQUAT_SPEED: f32 = DEFAULT_LANDING_SQUAT_SPEED * 1.3;

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
	Walk,
	Squat,
	Jump,
	Tuck,
	FixedTuck,
	TuckedFlip,
	TwoFootTuckedFlip,
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
		AnimationMode::Walk => animate_walk(&config, &mut rig, &mut armature, &mut limbs, t),
		AnimationMode::Squat => {
			animate_squat(&config, &mut debug, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::Jump => {
			animate_jump(&config, &mut debug, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::Tuck => animate_tuck(&config, &mut rig, &mut armature, &mut limbs, t),
		AnimationMode::FixedTuck => {
			animate_fixed_tuck(&config, &mut rig, &mut armature, &mut limbs)
		}
		AnimationMode::TuckedFlip => {
			animate_tucked_flip(&config, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::TwoFootTuckedFlip => {
			animate_two_foot_tucked_flip(&config, &mut debug, &mut rig, &mut armature, &mut limbs, t)
		}
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
	let effects = Run::default().apply(rig.as_mut(), t * RUN_CYCLE_SPEED);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_walk(
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
	let effects = Walk::default().apply(rig.as_mut(), t * WALK_CYCLE_SPEED);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_tuck(
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
	let progress = (t * TUCK_CYCLE_SPEED).rem_euclid(1.0);
	let effects = Tuck::<HumanoidV0Rig>::default().apply(rig.as_mut(), progress);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_fixed_tuck(
	config: &CharacterConfig,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs);
	let effects = FixedTuck::<HumanoidV0Rig>::default().apply(rig.as_mut(), 0.0);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_tucked_flip(
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
	let progress = (t * FRONT_FLIP_CYCLE_SPEED).rem_euclid(1.0);
	let effects = TuckedFlip::<HumanoidV0Rig>::default().apply(rig.as_mut(), progress);
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
	let squat = Squat::<HumanoidV0Rig>::for_loop(squat_half_speed, squat_half_speed);
	let squat_progress = t * SQUAT_CYCLE_SPEED;
	let effects = squat.apply(&mut rig, squat_progress);
	apply_effects(config.transform, effects, armature);

	if debug.0.should_log(t) {
		let phase = squat.cycle_phase(squat_progress);
		let lengths = rig.segment_lengths();
		let drop = squat.vertical_drop(squat_progress, lengths);
		let rest_by_bone = rest_transforms(limbs);
		let move_label = effects
			.r#move
			.map(|t| crozon_rigs::debug::format_vec3(t.translation))
			.unwrap_or_else(|| "none".into());
		let header = vec![
			format!("t={t:.2}s phase={phase:.3}"),
			format!(
				"envelope: depth={:.3} femur_swing={:.3} shin_flex={:.3} root_swing={:.3} vertical_drop={:.4}",
				squat.depth(squat_progress),
				squat.femur_swing(squat_progress),
				squat.shin_flex(squat_progress),
				squat.root_swing(squat_progress),
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

fn animate_two_foot_tucked_flip(
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
	let flip = TwoFootTuckedFlip::<HumanoidV0Rig>::default()
		.with_jump(
			TwoFootedJump::<HumanoidV0Rig>::default()
				.with_gravity(DEFAULT_GRAVITY)
				.with_jump_height(JUMP_HEIGHT)
				.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
				.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED),
		);
	let effects = flip.apply(&mut rig, t);
	apply_effects(config.transform, effects, armature);
	if debug.0.enabled {
		let lengths = rig.segment_lengths();
		let (segment, _) = flip.segment(lengths, t);
		if segment == malo_animations::animations::JumpSegment::Land || debug.0.should_log(t) {
			info!(
				"tucked flip: elapsed={:.3} pitch={:.3} y={:.3}",
				t,
				flip.flip_pitch_radians(lengths, t),
				flip.vertical_offset(lengths, t),
			);
		}
	}
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_jump(
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
	let jump = TwoFootedJump::<HumanoidV0Rig>::default()
		.with_gravity(DEFAULT_GRAVITY)
		.with_jump_height(JUMP_HEIGHT)
		.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
		.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED);
	let effects = jump.apply(&mut rig, t);
	apply_effects(config.transform, effects, armature);
	if debug.0.enabled {
		let lengths = rig.segment_lengths();
		let (segment, _) = jump.segment(lengths, t);
		if segment == malo_animations::animations::JumpSegment::Land || debug.0.should_log(t) {
			jump.log_landing_debug(&rig, t, "jump articulation debug");
		}
	}
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
