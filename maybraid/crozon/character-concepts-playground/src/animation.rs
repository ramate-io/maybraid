//! Lightweight body-rig animation for the concepts preview.
//!
//! This intentionally mirrors the older character playground instead of adding a
//! broader animation API. The concepts screen only needs enough motion to shake
//! out socket and skin-remap issues while the model surface is still changing.

use bevy::prelude::*;
pub use crozon_characters::ConceptAnimation;
use crozon_rigs::{
	forelimbed::ForelimbedRig,
	humanoid::HumanoidRig,
	quadruped::QuadrupedRig,
	rigs::{
		forelimbed_v0::ForelimbedV0Rig, humanoid_v0::HumanoidV0Rig, quadruped_v0::QuadrupedV0Rig,
	},
	BonePose, Name as RigName,
};
use malo_animations::{
	animations::{
		DorsoventralUndulation, Flapping, Gallop, Jab, LateralUndulation, QuadrupedRun, Run,
		Soaring, Tuck, TuckedFlip, TwoFootedJump, TwoFootedTuckedFlip, Walk, DEFAULT_GRAVITY,
		DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED,
	},
	Animation, Effects,
};

use crate::skinning::{BoneMap, CharacterRig, RigSkeletonKind};

const RUN_CYCLE_SPEED: f32 = 0.5;
const WALK_CYCLE_SPEED: f32 = 0.35;
const QUADRUPED_RUN_CYCLE_SPEED: f32 = 0.55;
const GALLOP_CYCLE_SPEED: f32 = 0.35;
const TUCK_CYCLE_SPEED: f32 = 0.6;
const FRONT_FLIP_CYCLE_SPEED: f32 = 0.85;
const JAB_CYCLE_SPEED: f32 = 0.9;
const JUMP_HEIGHT: f32 = 1.5;
const JUMP_PRE_SQUAT_SPEED: f32 = DEFAULT_PRE_SQUAT_SPEED * 1.2;
const JUMP_LANDING_SQUAT_SPEED: f32 = DEFAULT_LANDING_SQUAT_SPEED * 1.3;

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
		(Entity, &BoneMap, &CharacterRig),
		(
			With<CharacterRig>,
			With<AnimatedBodyRig>,
			Without<HumanoidV0Rig>,
			Without<QuadrupedV0Rig>,
			Without<ForelimbedV0Rig>,
		),
	>,
	transforms: Query<&Transform>,
) {
	let Ok((rig_entity, bone_map, character_rig)) = rig_roots.single() else {
		return;
	};
	if bone_map.by_name.is_empty() {
		return;
	}

	match character_rig.skeleton {
		RigSkeletonKind::Humanoid => {
			let humanoid = HumanoidV0Rig::imported();
			for bone in humanoid.animation_bones() {
				insert_limb_animator(&mut commands, bone_map, &transforms, bone);
			}
			commands.entity(rig_entity).insert(humanoid);
		}
		RigSkeletonKind::Quadruped => {
			let quadruped = QuadrupedV0Rig::imported();
			for bone in quadruped.animation_bones() {
				insert_limb_animator(&mut commands, bone_map, &transforms, bone);
			}
			commands.entity(rig_entity).insert(quadruped);
		}
		RigSkeletonKind::Forelimbed => {
			let forelimbed = ForelimbedV0Rig::imported();
			for bone in ForelimbedRig::animation_bones(&forelimbed) {
				insert_limb_animator(&mut commands, bone_map, &transforms, bone);
			}
			commands.entity(rig_entity).insert(forelimbed);
		}
		RigSkeletonKind::Neck => {}
	}
}

fn insert_limb_animator(
	commands: &mut Commands,
	bone_map: &BoneMap,
	transforms: &Query<&Transform>,
	bone: RigName,
) {
	let Some(&entity) = bone_map.by_name.get(bone.as_str()) else {
		return;
	};
	let Ok(transform) = transforms.get(entity) else {
		return;
	};
	commands.entity(entity).insert(LimbAnimator { bone, rest: *transform });
}

pub fn animate_body_rig(
	config: Res<crate::preview::ConceptPreviewConfig>,
	mut humanoid_rig: Query<
		&mut HumanoidV0Rig,
		(With<AnimatedBodyRig>, Without<QuadrupedV0Rig>, Without<ForelimbedV0Rig>),
	>,
	mut quadruped_rig: Query<
		&mut QuadrupedV0Rig,
		(With<AnimatedBodyRig>, Without<HumanoidV0Rig>, Without<ForelimbedV0Rig>),
	>,
	mut forelimbed_rig: Query<
		&mut ForelimbedV0Rig,
		(With<AnimatedBodyRig>, Without<HumanoidV0Rig>, Without<QuadrupedV0Rig>),
	>,
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

	let t = time.elapsed_secs();

	if let Ok(mut rig) = humanoid_rig.single_mut() {
		marshal_limbs_into_pose(rig.pose_mut(), &mut limbs);
		let effects = apply_humanoid_animation(animation, rig.as_mut(), t);
		apply_effects(effects, &mut armature);
		marshal_pose_to_limbs(rig.pose(), &mut limbs);
		return;
	}

	if let Ok(mut rig) = quadruped_rig.single_mut() {
		marshal_limbs_into_pose(rig.pose_mut(), &mut limbs);
		let effects = apply_quadruped_animation(animation, rig.as_mut(), t);
		apply_effects(effects, &mut armature);
		marshal_pose_to_limbs(rig.pose(), &mut limbs);
		return;
	}

	let Ok(mut rig) = forelimbed_rig.single_mut() else {
		return;
	};
	marshal_limbs_into_pose(rig.pose_mut(), &mut limbs);
	let effects = apply_forelimbed_animation(animation, rig.as_mut(), t);
	apply_effects(effects, &mut armature);
	marshal_pose_to_limbs(rig.pose(), &mut limbs);
}

fn apply_humanoid_animation(
	animation: ConceptAnimation,
	rig: &mut HumanoidV0Rig,
	t: f32,
) -> Effects {
	match animation {
		ConceptAnimation::Still => Effects::default(),
		ConceptAnimation::Walk => Walk::default().apply(rig, t * WALK_CYCLE_SPEED),
		ConceptAnimation::Run => Run::default().apply(rig, t * RUN_CYCLE_SPEED),
		ConceptAnimation::Gallop => Effects::default(),
		ConceptAnimation::Jump => TwoFootedJump::<HumanoidV0Rig>::default()
			.with_gravity(DEFAULT_GRAVITY)
			.with_jump_height(JUMP_HEIGHT)
			.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
			.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED)
			.apply(rig, t),
		ConceptAnimation::Tuck => {
			let progress = (t * TUCK_CYCLE_SPEED).rem_euclid(1.0);
			Tuck::<HumanoidV0Rig>::default().apply(rig, progress)
		}
		ConceptAnimation::TuckedFlip => {
			let progress = (t * FRONT_FLIP_CYCLE_SPEED).rem_euclid(1.0);
			TuckedFlip::<HumanoidV0Rig>::default().apply(rig, progress)
		}
		ConceptAnimation::TwoFootedTuckedFlip => TwoFootedTuckedFlip::<HumanoidV0Rig>::default()
			.with_jump(
				TwoFootedJump::<HumanoidV0Rig>::default()
					.with_gravity(DEFAULT_GRAVITY)
					.with_jump_height(JUMP_HEIGHT)
					.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
					.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED),
			)
			.apply(rig, t),
		ConceptAnimation::Soaring => Soaring::default().apply(rig, t),
		ConceptAnimation::Flapping => Flapping::default().apply(rig, t),
		ConceptAnimation::Jab => {
			let progress = (t * JAB_CYCLE_SPEED).rem_euclid(1.0);
			Jab::<HumanoidV0Rig>::default().apply(rig, progress)
		}
		ConceptAnimation::LateralUndulation | ConceptAnimation::DorsoventralUndulation => {
			Effects::default()
		}
	}
}

fn apply_quadruped_animation(
	animation: ConceptAnimation,
	rig: &mut QuadrupedV0Rig,
	t: f32,
) -> Effects {
	match animation {
		ConceptAnimation::Run => QuadrupedRun::default().apply(rig, t * QUADRUPED_RUN_CYCLE_SPEED),
		ConceptAnimation::Gallop => Gallop::default().apply(rig, t * GALLOP_CYCLE_SPEED),
		_ => Effects::default(),
	}
}

fn apply_forelimbed_animation(
	animation: ConceptAnimation,
	rig: &mut ForelimbedV0Rig,
	t: f32,
) -> Effects {
	match animation {
		ConceptAnimation::LateralUndulation => LateralUndulation::default().apply(rig, t),
		ConceptAnimation::DorsoventralUndulation => DorsoventralUndulation::default().apply(rig, t),
		_ => Effects::default(),
	}
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
	pose: &mut crozon_rigs::RigPose,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
) {
	for (_, animator) in limbs.iter_mut() {
		pose.insert(BonePose::new(animator.bone.clone(), animator.rest));
	}
}

fn marshal_pose_to_limbs(
	pose: &crozon_rigs::RigPose,
	limbs: &mut Query<(&mut Transform, &LimbAnimator)>,
) {
	for (mut transform, animator) in limbs.iter_mut() {
		if let Some(bone_pose) = pose.get(&animator.bone) {
			*transform = bone_pose.transform;
		}
	}
}
