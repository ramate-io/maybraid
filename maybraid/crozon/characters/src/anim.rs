//! Clip identity on a rig member, plus a latest-wins mailbox that transitions
//! from the last applied [`RigPose`].
//!
//! Parallel to [`MaterialRefRoot`]: insert [`AnimRefRoot`] on the body-rig host.
//! Sampling ticks every frame; [`Changed`] clip (detected via the mailbox) starts
//! a [`malo_animations::animations::Transition`]-style blend from current output.

use bevy::prelude::*;
use crozon_rigs::{
	forelimbed::ForelimbedRig,
	rigs::{
		forelimbed_v0::ForelimbedV0Rig, humanoid_v0::HumanoidV0Rig, quadruped_v0::QuadrupedV0Rig,
	},
	BonePose, Name as RigName, RigPose,
};
use malo_animations::{
	animations::{
		DorsoventralUndulation, Flapping, Gallop, Jab, LateralUndulation, QuadrupedRun, Run,
		Soaring, Tuck, TuckedFlip, TwoFootedJump, TwoFootedTuckedFlip, Walk, DEFAULT_GRAVITY,
		DEFAULT_LANDING_SQUAT_SPEED, DEFAULT_PRE_SQUAT_SPEED,
	},
	Animation, Effects,
};

use crate::concepts::ConceptAnimation;
use crate::rig::{bone_map_ready, BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};

const RUN_CYCLE_SPEED: f32 = 0.5;
const WALK_CYCLE_SPEED: f32 = 0.35;
const GALLOP_CYCLE_SPEED: f32 = 0.35;
const TUCK_CYCLE_SPEED: f32 = 0.6;
const FRONT_FLIP_CYCLE_SPEED: f32 = 0.85;
const JAB_CYCLE_SPEED: f32 = 0.9;
const JUMP_HEIGHT: f32 = 1.5;
const JUMP_PRE_SQUAT_SPEED: f32 = DEFAULT_PRE_SQUAT_SPEED * 1.2;
const JUMP_LANDING_SQUAT_SPEED: f32 = DEFAULT_LANDING_SQUAT_SPEED * 1.3;
const BLEND_DURATION: f32 = 0.15;

/// Runtime clip identity. Not the concepts-screen catalog ([`ConceptAnimation`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AnimId {
	#[default]
	Still,
	Walk,
	Run,
	Gallop,
	Jump,
	Tuck,
	TuckedFlip,
	TwoFootedTuckedFlip,
	Soaring,
	Flapping,
	Jab,
	LateralUndulation,
	DorsoventralUndulation,
}

impl AnimId {
	pub const fn default_speed(self) -> f32 {
		match self {
			Self::Still => 1.0,
			Self::Walk => WALK_CYCLE_SPEED,
			Self::Run => RUN_CYCLE_SPEED,
			Self::Gallop => GALLOP_CYCLE_SPEED,
			Self::Jump => 1.0,
			Self::Tuck => TUCK_CYCLE_SPEED,
			Self::TuckedFlip => FRONT_FLIP_CYCLE_SPEED,
			Self::TwoFootedTuckedFlip => 1.0,
			Self::Soaring => 1.0,
			Self::Flapping => 1.0,
			Self::Jab => JAB_CYCLE_SPEED,
			Self::LateralUndulation => 1.0,
			Self::DorsoventralUndulation => 1.0,
		}
	}
}

impl From<ConceptAnimation> for AnimId {
	fn from(value: ConceptAnimation) -> Self {
		match value {
			ConceptAnimation::Still => Self::Still,
			ConceptAnimation::Walk => Self::Walk,
			ConceptAnimation::Run => Self::Run,
			ConceptAnimation::Gallop => Self::Gallop,
			ConceptAnimation::Jump => Self::Jump,
			ConceptAnimation::Tuck => Self::Tuck,
			ConceptAnimation::TuckedFlip => Self::TuckedFlip,
			ConceptAnimation::TwoFootedTuckedFlip => Self::TwoFootedTuckedFlip,
			ConceptAnimation::Soaring => Self::Soaring,
			ConceptAnimation::Flapping => Self::Flapping,
			ConceptAnimation::Jab => Self::Jab,
			ConceptAnimation::LateralUndulation => Self::LateralUndulation,
			ConceptAnimation::DorsoventralUndulation => Self::DorsoventralUndulation,
		}
	}
}

/// Clip + playback speed on a rig member.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AnimRef {
	pub clip: AnimId,
	pub speed: f32,
}

impl AnimRef {
	pub fn new(clip: AnimId) -> Self {
		Self { clip, speed: clip.default_speed() }
	}

	pub fn still() -> Self {
		Self::new(AnimId::Still)
	}
}

impl From<ConceptAnimation> for AnimRef {
	fn from(value: ConceptAnimation) -> Self {
		Self::new(AnimId::from(value))
	}
}

impl Default for AnimRef {
	fn default() -> Self {
		Self::still()
	}
}

/// BSN / ECS identity: play this clip on this rig member.
#[derive(Component, Clone, Copy, Debug, PartialEq, Default)]
pub struct AnimRefRoot(pub AnimRef);

/// Marks a bone owned by the animation mailbox (pose apply skips its rotation).
#[derive(Component, Clone)]
pub struct AnimBone {
	pub name: RigName,
	pub rest: Transform,
}

/// Last sampled pose + in-flight transition. Latest [`AnimRefRoot`] wins.
#[derive(Component, Clone)]
pub struct AnimMailbox {
	pub output: RigPose,
	last: Option<AnimId>,
	clip_progress: f32,
	blend_progress: f32,
	from_pose: RigPose,
	bind_transform: Transform,
}

impl AnimMailbox {
	fn new(bind_transform: Transform) -> Self {
		Self {
			output: RigPose::new(),
			last: None,
			clip_progress: 0.0,
			blend_progress: 1.0,
			from_pose: RigPose::new(),
			bind_transform,
		}
	}

	fn blending(&self) -> bool {
		self.blend_progress < 1.0
	}
}

/// Insert typed rigs, [`AnimBone`]s, and [`AnimMailbox`] once the bone map is ready.
pub fn prepare_anim_mailbox(
	mut commands: Commands,
	hosts: Query<
		(Entity, &AnimRefRoot, &BoneMap, &CharacterRig, &Transform),
		(Without<AnimMailbox>, With<CharacterRig>),
	>,
	transforms: Query<&Transform>,
) {
	for (entity, _root, bone_map, character_rig, transform) in &hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if !bone_map_ready(bone_map, character_rig.skeleton) {
			continue;
		}

		let bone_names = match character_rig.skeleton {
			RigSkeletonKind::Humanoid => {
				let rig = HumanoidV0Rig::imported();
				let names = rig.animation_bones();
				commands.entity(entity).insert(rig);
				names
			}
			RigSkeletonKind::Quadruped => {
				let rig = QuadrupedV0Rig::imported();
				let names = rig.animation_bones();
				commands.entity(entity).insert(rig);
				names
			}
			RigSkeletonKind::Forelimbed => {
				let rig = ForelimbedV0Rig::imported();
				let names = ForelimbedRig::animation_bones(&rig);
				commands.entity(entity).insert(rig);
				names
			}
			RigSkeletonKind::Neck => continue,
		};

		for name in bone_names {
			let Some(&bone_entity) = bone_map.by_name.get(name.as_str()) else {
				continue;
			};
			let Ok(bone_tf) = transforms.get(bone_entity) else {
				continue;
			};
			commands
				.entity(bone_entity)
				.insert(AnimBone { name: name.clone(), rest: *bone_tf });
		}

		commands.entity(entity).insert(AnimMailbox::new(*transform));
	}
}

/// Sample the requested clip; on clip change, blend from the last output pose.
pub fn tick_anim_mailbox(
	time: Res<Time>,
	mut hosts: Query<
		(
			&AnimRefRoot,
			&mut AnimMailbox,
			&BoneMap,
			&CharacterRig,
			&mut Transform,
			Option<&mut HumanoidV0Rig>,
			Option<&mut QuadrupedV0Rig>,
			Option<&mut ForelimbedV0Rig>,
		),
		(With<AnimMailbox>, Without<AnimBone>),
	>,
	mut bones: Query<(&AnimBone, &mut Transform), Without<AnimMailbox>>,
) {
	let dt = time.delta_secs();
	for (
		root,
		mut mailbox,
		bone_map,
		character_rig,
		mut armature,
		humanoid,
		quadruped,
		forelimbed,
	) in &mut hosts
	{
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}

		let requested = root.0.clip;
		if mailbox.last != Some(requested) {
			mailbox.from_pose = if mailbox.output.is_empty() {
				rest_pose(bone_map, &bones)
			} else {
				mailbox.output.clone()
			};
			mailbox.blend_progress = 0.0;
			mailbox.clip_progress = 0.0;
			mailbox.last = Some(requested);
		}

		mailbox.clip_progress += dt * root.0.speed;
		if mailbox.blending() {
			mailbox.blend_progress = (mailbox.blend_progress + dt / BLEND_DURATION).min(1.0);
		}

		let rest = rest_pose(bone_map, &bones);
		let (sampled, effects) = match character_rig.skeleton {
			RigSkeletonKind::Humanoid => {
				let Some(mut rig) = humanoid else {
					continue;
				};
				seed_rig(&mut rig.pose, &rest);
				let effects = sample_humanoid(requested, &mut rig, mailbox.clip_progress);
				(rig.pose.clone(), effects)
			}
			RigSkeletonKind::Quadruped => {
				let Some(mut rig) = quadruped else {
					continue;
				};
				seed_rig(&mut rig.pose, &rest);
				let effects = sample_quadruped(requested, &mut rig, mailbox.clip_progress);
				(rig.pose.clone(), effects)
			}
			RigSkeletonKind::Forelimbed => {
				let Some(mut rig) = forelimbed else {
					continue;
				};
				seed_rig(&mut rig.pose, &rest);
				let effects = sample_forelimbed(requested, &mut rig, mailbox.clip_progress);
				(rig.pose.clone(), effects)
			}
			RigSkeletonKind::Neck => continue,
		};

		let weight = smoothstep(mailbox.blend_progress);
		let pose = if mailbox.blending() || weight < 1.0 {
			RigPose::blend(&mailbox.from_pose, &sampled, weight)
		} else {
			sampled
		};
		write_pose(&pose, bone_map, &mut bones);
		apply_root_motion(&mut armature, mailbox.bind_transform, effects, weight);
		mailbox.output = pose;
	}
}

fn rest_pose(
	bone_map: &BoneMap,
	bones: &Query<(&AnimBone, &mut Transform), Without<AnimMailbox>>,
) -> RigPose {
	let mut pose = RigPose::new();
	for entity in bone_map.by_name.values() {
		let Ok((bone, _)) = bones.get(*entity) else {
			continue;
		};
		pose.insert(BonePose::new(bone.name.clone(), bone.rest));
	}
	pose
}

fn seed_rig(pose: &mut RigPose, rest: &RigPose) {
	for (_, bone) in rest.iter() {
		pose.insert(bone.clone());
	}
}

fn write_pose(
	pose: &RigPose,
	bone_map: &BoneMap,
	bones: &mut Query<(&AnimBone, &mut Transform), Without<AnimMailbox>>,
) {
	for (name, entity) in &bone_map.by_name {
		let Ok((anim_bone, mut transform)) = bones.get_mut(*entity) else {
			continue;
		};
		if let Some(bone_pose) = pose.get(&anim_bone.name) {
			*transform = bone_pose.transform;
		} else if let Some(bone_pose) = pose.get(&RigName::from(name.as_str())) {
			*transform = bone_pose.transform;
		}
	}
}

fn apply_root_motion(armature: &mut Transform, bind: Transform, effects: Effects, weight: f32) {
	*armature = bind;
	let Some(offset) = effects.r#move else {
		return;
	};
	let t = lerp_transform(Transform::IDENTITY, offset, weight);
	armature.translation += t.translation;
	armature.rotation = t.rotation * armature.rotation;
	armature.scale *= t.scale;
}

fn lerp_transform(a: Transform, b: Transform, t: f32) -> Transform {
	Transform {
		translation: a.translation.lerp(b.translation, t),
		rotation: a.rotation.slerp(b.rotation, t),
		scale: a.scale.lerp(b.scale, t),
	}
}

fn smoothstep(t: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

fn sample_humanoid(clip: AnimId, rig: &mut HumanoidV0Rig, progress: f32) -> Effects {
	match clip {
		AnimId::Still => Effects::default(),
		AnimId::Walk => Walk::default().apply(rig, progress),
		AnimId::Run => Run::default().apply(rig, progress),
		AnimId::Gallop => Effects::default(),
		AnimId::Jump => TwoFootedJump::<HumanoidV0Rig>::default()
			.with_gravity(DEFAULT_GRAVITY)
			.with_jump_height(JUMP_HEIGHT)
			.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
			.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED)
			.apply(rig, progress),
		AnimId::Tuck => Tuck::<HumanoidV0Rig>::default().apply(rig, progress.rem_euclid(1.0)),
		AnimId::TuckedFlip => {
			TuckedFlip::<HumanoidV0Rig>::default().apply(rig, progress.rem_euclid(1.0))
		}
		AnimId::TwoFootedTuckedFlip => TwoFootedTuckedFlip::<HumanoidV0Rig>::default()
			.with_jump(
				TwoFootedJump::<HumanoidV0Rig>::default()
					.with_gravity(DEFAULT_GRAVITY)
					.with_jump_height(JUMP_HEIGHT)
					.with_pre_squat_speed(JUMP_PRE_SQUAT_SPEED)
					.with_landing_squat_speed(JUMP_LANDING_SQUAT_SPEED),
			)
			.apply(rig, progress),
		AnimId::Soaring => Soaring::default().apply(rig, progress),
		AnimId::Flapping => Flapping::default().apply(rig, progress),
		AnimId::Jab => Jab::<HumanoidV0Rig>::default().apply(rig, progress.rem_euclid(1.0)),
		AnimId::LateralUndulation | AnimId::DorsoventralUndulation => Effects::default(),
	}
}

fn sample_quadruped(clip: AnimId, rig: &mut QuadrupedV0Rig, progress: f32) -> Effects {
	match clip {
		AnimId::Run => QuadrupedRun::default().apply(rig, progress),
		AnimId::Gallop => Gallop::default().apply(rig, progress),
		_ => Effects::default(),
	}
}

fn sample_forelimbed(clip: AnimId, rig: &mut ForelimbedV0Rig, progress: f32) -> Effects {
	match clip {
		AnimId::LateralUndulation => LateralUndulation::default().apply(rig, progress),
		AnimId::DorsoventralUndulation => DorsoventralUndulation::default().apply(rig, progress),
		_ => Effects::default(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn concept_animation_maps_to_anim_ref() {
		let walk = AnimRef::from(ConceptAnimation::Walk);
		assert_eq!(walk.clip, AnimId::Walk);
		assert_eq!(walk.speed, WALK_CYCLE_SPEED);
	}
}
