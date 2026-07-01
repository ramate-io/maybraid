//! Lightweight body-rig animation for the concepts preview.
//!
//! This intentionally mirrors the older character playground instead of adding a
//! broader animation API. The concepts screen only needs enough motion to shake
//! out socket and skin-remap issues while the model surface is still changing.

use bevy::prelude::*;
use clap::ValueEnum;
use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, BonePose, Name as RigName};
use malo_animations::{
	animations::{
		Run, Tuck, TuckedFlip, TwoFootedJump, TwoFootedTuckedFlip, Walk, DEFAULT_GRAVITY,
		DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED,
	},
	Animation, Effects,
};

use crate::skinning::{BoneMap, CharacterRig};

const RUN_CYCLE_SPEED: f32 = 0.5;
const WALK_CYCLE_SPEED: f32 = 0.35;
const TUCK_CYCLE_SPEED: f32 = 0.6;
const FRONT_FLIP_CYCLE_SPEED: f32 = 0.85;
const JUMP_HEIGHT: f32 = 1.5;
const JUMP_PRE_SQUAT_SPEED: f32 = DEFAULT_PRE_SQUAT_SPEED * 1.2;
const JUMP_LANDING_SQUAT_SPEED: f32 = DEFAULT_LANDING_SQUAT_SPEED * 1.3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ConceptAnimation {
	#[default]
	Still,
	Walk,
	Run,
	Jump,
	Tuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
}

impl ConceptAnimation {
	pub const fn label(self) -> &'static str {
		match self {
			Self::Still => "still",
			Self::Walk => "walk",
			Self::Run => "run",
			Self::Jump => "jump",
			Self::Tuck => "tuck",
			Self::TuckedFlip => "tucked-flip",
			Self::TwoFootedTuckedFlip => "two-footed-tucked-flip",
		}
	}
}

/// Marks the body rig as the only rig animated in this pass.
#[derive(Component)]
pub struct AnimatedBodyRig;

/// Armature transform before animation root-motion effects are applied.
#[derive(Component, Clone, Copy)]
pub struct BodyRigBindTransform(pub Transform);

#[derive(Component)]
pub struct LimbAnimator {
	pub bone: RigName,
	pub rest: Transform,
}

pub fn init_limb_animators(
	mut commands: Commands,
	rig_roots: Query<
		(Entity, &BoneMap),
		(With<CharacterRig>, With<AnimatedBodyRig>, Without<HumanoidV0Rig>),
	>,
	transforms: Query<&Transform>,
) {
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
		// Capture local bind transforms once; animation samples are reapplied from
		// this baseline so procedural motion does not accumulate over time.
		commands.entity(entity).insert(LimbAnimator { bone, rest: *transform });
	}

	commands.entity(rig_entity).insert(humanoid);
}

pub fn animate_body_rig(
	config: Res<crate::preview::ConceptPreviewConfig>,
	mut rig: Query<&mut HumanoidV0Rig, With<AnimatedBodyRig>>,
	mut armature: Query<
		(&BodyRigBindTransform, &mut Transform),
		(With<AnimatedBodyRig>, Without<LimbAnimator>),
	>,
	mut limbs: Query<(&mut Transform, &LimbAnimator)>,
	time: Res<Time>,
) {
	let animation = config.animation();
	if animation == ConceptAnimation::Still {
		return;
	}

	let Ok(mut rig) = rig.single_mut() else {
		return;
	};
	let t = time.elapsed_secs();

	marshal_limbs_into_pose(&mut rig, &mut limbs);
	let effects = match animation {
		ConceptAnimation::Still => Effects::default(),
		ConceptAnimation::Walk => Walk::default().apply(rig.as_mut(), t * WALK_CYCLE_SPEED),
		ConceptAnimation::Run => Run::default().apply(rig.as_mut(), t * RUN_CYCLE_SPEED),
		ConceptAnimation::Jump => TwoFootedJump::<HumanoidV0Rig>::default()
			.with_gravity(DEFAULT_GRAVITY)
			.with_jump_height(JUMP_HEIGHT)
			.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
			.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED)
			.apply(rig.as_mut(), t),
		ConceptAnimation::Tuck => {
			let progress = (t * TUCK_CYCLE_SPEED).rem_euclid(1.0);
			Tuck::<HumanoidV0Rig>::default().apply(rig.as_mut(), progress)
		}
		ConceptAnimation::TuckedFlip => {
			let progress = (t * FRONT_FLIP_CYCLE_SPEED).rem_euclid(1.0);
			TuckedFlip::<HumanoidV0Rig>::default().apply(rig.as_mut(), progress)
		}
		ConceptAnimation::TwoFootedTuckedFlip => TwoFootedTuckedFlip::<HumanoidV0Rig>::default()
			.with_jump(
				TwoFootedJump::<HumanoidV0Rig>::default()
					.with_gravity(DEFAULT_GRAVITY)
					.with_jump_height(JUMP_HEIGHT)
					.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
					.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED),
			)
			.apply(rig.as_mut(), t),
	};
	apply_effects(effects, &mut armature);
	marshal_pose_to_limbs(&rig, &mut limbs);
}

fn apply_effects(
	effects: Effects,
	armature: &mut Query<
		(&BodyRigBindTransform, &mut Transform),
		(With<AnimatedBodyRig>, Without<LimbAnimator>),
	>,
) {
	let Ok((bind, mut transform)) = armature.single_mut() else {
		return;
	};

	*transform = bind.0;
	if let Some(offset) = effects.r#move {
		transform.translation += offset.translation;
		transform.rotation = offset.rotation * transform.rotation;
		transform.scale *= offset.scale;
	}
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
