use bevy::prelude::*;
use character_animations::{
	animations::{
		FixedTuck, Run, Squat, SquatDescent, Tuck, TuckedFlip, TwoFootedJump, TwoFootedTuckedFlip,
		UprightLeap, Walk, DEFAULT_DESCENT_SPEED, DEFAULT_GRAVITY, DEFAULT_LANDING_SQUAT_SPEED,
		DEFAULT_PRE_SQUAT_SPEED,
	},
	Animation, Effects,
};
use character_rigs::{
	articulation::compose_parent_rotation,
	authoring::humanoid_bone_axis,
	debug::{format_rigged_axis, log_bind_pose, RigPoseDebug},
	rigs::humanoid_v0::HumanoidV0Rig,
	Name as RigName,
};
use clap::ValueEnum;
use log::info;

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const RUN_CYCLE_SPEED: f32 = 1.4;
const WALK_CYCLE_SPEED: f32 = 0.9;
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
	SquatDescent,
	Jump,
	Leap,
	Tuck,
	FixedTuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
}

#[derive(Resource)]
pub struct AnimationArticulationDebug(RigPoseDebug);

#[derive(Resource, Debug, Clone)]
pub struct AnimationPlayback {
	pub paused: bool,
	pub speed: f32,
	/// When false, the sample clock stops at one second.
	pub looping: bool,
	pub elapsed: f32,
	/// Set by `/character playback --progress`. Consumed on the next sample.
	pub scrub: Option<f32>,
	/// Added to the sample clock. `0.5` is the opposite gait phase.
	pub phase: f32,
	/// Scales femur and shin rest translations before sampling.
	pub leg_scale: f32,
	/// Bone name for the axis gizmo. Empty hides it.
	pub joint: String,
	/// Draw the effective-rest direction next to the posed bone.
	pub show_rest: bool,
	/// When set, replace that bone's clip rotation with these anatomical degrees.
	pub joint_degrees: Option<JointDegrees>,
}

#[derive(Debug, Clone, Copy)]
pub struct JointDegrees {
	pub flexion: f32,
	pub lateral: f32,
	pub axial: f32,
}

impl Default for AnimationPlayback {
	fn default() -> Self {
		Self {
			paused: false,
			speed: 1.0,
			looping: true,
			elapsed: 0.0,
			scrub: None,
			phase: 0.0,
			leg_scale: 1.0,
			joint: String::new(),
			show_rest: true,
			joint_degrees: None,
		}
	}
}

impl AnimationPlayback {
	pub fn advance(&mut self, delta_seconds: f32) {
		if let Some(progress) = self.scrub.take() {
			self.elapsed = progress.max(0.0);
			return;
		}
		if self.paused {
			return;
		}
		self.elapsed += delta_seconds * self.speed;
		if !self.looping {
			self.elapsed = self.elapsed.min(1.0);
		}
	}
}

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
	mut playback: ResMut<AnimationPlayback>,
	mut rig: Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	mut armature: Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	mut limbs: Query<(&mut Transform, &LimbAnimator)>,
	time: Res<Time>,
) {
	playback.advance(time.delta_secs());
	let t = playback.elapsed + playback.phase;
	match config.animation {
		AnimationMode::Run => {
			animate_run(&config, &playback, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::Walk => {
			animate_walk(&config, &playback, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::Squat => {
			animate_squat(&config, &playback, &mut debug, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::SquatDescent => animate_squat_descent(
			&config,
			&playback,
			&mut debug,
			&mut rig,
			&mut armature,
			&mut limbs,
			t,
		),
		AnimationMode::Jump => {
			animate_jump(&config, &playback, &mut debug, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::Leap => {
			animate_leap(&config, &playback, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::Tuck => {
			animate_tuck(&config, &playback, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::FixedTuck => {
			animate_fixed_tuck(&config, &playback, &mut rig, &mut armature, &mut limbs)
		}
		AnimationMode::TuckedFlip => {
			animate_tucked_flip(&config, &playback, &mut rig, &mut armature, &mut limbs, t)
		}
		AnimationMode::TwoFootedTuckedFlip => animate_two_footed_tucked_flip(
			&config,
			&playback,
			&mut debug,
			&mut rig,
			&mut armature,
			&mut limbs,
			t,
		),
	}
	apply_joint_preview(&playback, &mut rig, &mut limbs);
}

fn animate_run(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let effects = Run::default().apply(rig.as_mut(), t * RUN_CYCLE_SPEED);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_walk(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let effects = Walk::default().apply(rig.as_mut(), t * WALK_CYCLE_SPEED);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_tuck(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let progress = (t * TUCK_CYCLE_SPEED).rem_euclid(1.0);
	let effects = Tuck::default().apply(rig.as_mut(), progress);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_fixed_tuck(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let effects = FixedTuck::default().apply(rig.as_mut(), 0.0);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_tucked_flip(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let progress = (t * FRONT_FLIP_CYCLE_SPEED).rem_euclid(1.0);
	let effects = TuckedFlip::default().apply(rig.as_mut(), progress);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_squat(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	debug: &mut AnimationArticulationDebug,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let squat_half_speed = 2.0 * SQUAT_CYCLE_SPEED;
	let squat = Squat::for_loop(squat_half_speed, squat_half_speed);
	let squat_progress = t * SQUAT_CYCLE_SPEED;
	let effects = squat.apply(&mut rig, squat_progress);
	apply_effects(config.transform, effects, armature);

	if debug.0.should_log(t) {
		let phase = squat.cycle_phase(squat_progress);
		let lengths = rig.segment_lengths;
		let drop = squat.vertical_drop(squat_progress, lengths);
		let move_label = if effects.is_identity() {
			"none".to_string()
		} else {
			character_rigs::debug::format_vec3(effects.0.translation)
		};
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

		for line in &header {
			info!("{line}");
		}
		for name in DEBUG_BONES {
			let tip = rig.rotation(name) * Vec3::Y;
			info!("[{name}] tip={}", character_rigs::debug::format_vec3(tip));
		}
	}

	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_squat_descent(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	debug: &mut AnimationArticulationDebug,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let descent = SquatDescent::default();
	let progress = (t * DEFAULT_DESCENT_SPEED).clamp(0.0, 1.0);
	let effects = descent.apply(&mut rig, progress);
	apply_effects(config.transform, effects, armature);

	if debug.0.should_log(t) {
		info!(
			"t={t:.2}s depth={:.3} femur.L={:.3}",
			descent.depth(progress),
			rig.posed_angle("femur.L"),
		);
	}

	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_two_footed_tucked_flip(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	debug: &mut AnimationArticulationDebug,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let flip = TwoFootedTuckedFlip::default().with_jump(
		TwoFootedJump::default()
			.with_gravity(DEFAULT_GRAVITY)
			.with_jump_height(JUMP_HEIGHT)
			.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
			.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED),
	);
	let effects = flip.apply(&mut rig, t);
	apply_effects(config.transform, effects, armature);
	if debug.0.enabled {
		let lengths = rig.segment_lengths;
		let (segment, _) = flip.segment(lengths, t);
		if segment == character_animations::animations::JumpSegment::Land || debug.0.should_log(t) {
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

fn animate_leap(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let progress = t.clamp(0.0, 1.0);
	let effects = UprightLeap::default().apply(&mut rig, progress);
	apply_effects(config.transform, effects, armature);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_jump(
	config: &CharacterConfig,
	playback: &AnimationPlayback,
	debug: &mut AnimationArticulationDebug,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	armature: &mut Query<&mut Transform, (With<CharacterRig>, Without<LimbAnimator>)>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs, playback);
	let jump = TwoFootedJump::default()
		.with_gravity(DEFAULT_GRAVITY)
		.with_jump_height(JUMP_HEIGHT)
		.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
		.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED);
	let effects = jump.apply(&mut rig, t);
	apply_effects(config.transform, effects, armature);
	if debug.0.enabled {
		let lengths = rig.segment_lengths;
		let (segment, _) = jump.segment(lengths, t);
		if segment == character_animations::animations::JumpSegment::Land || debug.0.should_log(t) {
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

	*transform = effects.apply_to_bind(bind);
}

fn marshal_limbs_into_pose(
	rig: &mut HumanoidV0Rig,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
	playback: &AnimationPlayback,
) {
	let mut rest = rig.binding.effective_rest.clone();
	for (_, animator) in limbs.iter() {
		let Some(id) = rig.binding.definition.id(animator.bone.as_str()) else {
			continue;
		};
		if let Some(slot) = rest.local.get_mut(id.index()) {
			*slot = animator.rest;
			if playback.leg_scale != 1.0 && is_leg_bone(animator.bone.as_str()) {
				slot.translation *= playback.leg_scale;
			}
		}
	}
	rig.binding.refresh_rest(rest);
	rig.segment_lengths = rig.binding.metrics.humanoid_leg;
}

fn is_leg_bone(name: &str) -> bool {
	matches!(name, "femur.L" | "femur.R" | "shin.L" | "shin.R")
}

fn apply_joint_preview(
	playback: &AnimationPlayback,
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
) {
	let Some(degrees) = playback.joint_degrees else {
		return;
	};
	if playback.joint.is_empty() {
		return;
	}
	let Ok(rig) = rig.single() else {
		return;
	};
	let Some(id) = rig.binding.definition.id(&playback.joint) else {
		return;
	};
	let Some(rest) = rig.binding.effective_rest.get(id) else {
		return;
	};
	let rotation = compose_parent_rotation(
		rest.rotation,
		humanoid_bone_axis(&playback.joint),
		degrees.lateral.to_radians(),
		degrees.flexion.to_radians(),
		degrees.axial.to_radians(),
	);
	for (mut transform, animator) in limbs.iter_mut() {
		if animator.bone.as_str() == playback.joint {
			transform.rotation = rotation;
		}
	}
}

fn marshal_pose_to_limbs(rig: &HumanoidV0Rig, limbs: &mut Query<(&mut Transform, &LimbAnimator)>) {
	for (mut transform, animator) in limbs.iter_mut() {
		let Some(id) = rig.binding.definition.id(animator.bone.as_str()) else {
			continue;
		};
		if let Some(pose) = rig.pose.get(id) {
			if *transform != pose {
				*transform = pose;
			}
		}
	}
}

/// Bone-local axes are bright RGB. Parent-local axes are shorter and dimmer.
/// Character-space axes sit on the rig: +X right, +Y up, +Z fight-forward.
pub fn draw_authoring_gizmos(
	playback: Res<AnimationPlayback>,
	mut gizmos: Gizmos,
	bones: Query<(&GlobalTransform, &Transform, &LimbAnimator)>,
	rigs: Query<&GlobalTransform, With<CharacterRig>>,
) {
	if playback.joint.is_empty() {
		return;
	}
	if let Ok(rig) = rigs.single() {
		let origin = rig.translation();
		let character = rig.rotation();
		draw_axes(&mut gizmos, origin, character, 0.4, 1.0);
	}
	let opposite = opposite_bone(&playback.joint);
	for (global, local, limb) in &bones {
		let selected = limb.bone.as_str() == playback.joint;
		let compared = opposite.as_deref() == Some(limb.bone.as_str());
		if !selected && !compared {
			continue;
		}
		let origin = global.translation();
		let gain = if selected { 1.0 } else { 0.55 };
		let length = if selected { 0.22 } else { 0.16 };
		draw_axes(&mut gizmos, origin, global.rotation(), length, gain);
		let parent = global.rotation() * local.rotation.inverse();
		draw_axes(&mut gizmos, origin, parent, length * 0.65, gain * 0.45);
		if playback.show_rest {
			let rest_dir = parent * limb.rest.rotation * Vec3::Y * 0.28;
			let color =
				if selected { Color::srgb(1.0, 0.85, 0.2) } else { Color::srgb(0.75, 0.6, 0.15) };
			gizmos.line(origin, origin + rest_dir, color);
		}
		let segment = limb.rest.translation.length().max(0.05);
		let end = origin + global.rotation() * Vec3::Y * segment;
		gizmos.line(end - Vec3::X * 0.03, end + Vec3::X * 0.03, Color::srgb(1.0, 1.0, 1.0));
		gizmos.line(end - Vec3::Z * 0.03, end + Vec3::Z * 0.03, Color::srgb(1.0, 1.0, 1.0));
	}
}

fn opposite_bone(name: &str) -> Option<String> {
	if let Some(base) = name.strip_suffix(".L") {
		Some(format!("{base}.R"))
	} else {
		name.strip_suffix(".R").map(|base| format!("{base}.L"))
	}
}

fn draw_axes(gizmos: &mut Gizmos, origin: Vec3, rotation: Quat, length: f32, gain: f32) {
	gizmos.line(
		origin,
		origin + rotation * Vec3::X * length,
		Color::srgb(gain, 0.15 * gain, 0.15 * gain),
	);
	gizmos.line(
		origin,
		origin + rotation * Vec3::Y * length,
		Color::srgb(0.15 * gain, gain, 0.15 * gain),
	);
	gizmos.line(
		origin,
		origin + rotation * Vec3::Z * length,
		Color::srgb(0.2 * gain, 0.35 * gain, gain),
	);
}
