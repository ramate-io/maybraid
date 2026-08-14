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
use crozon_rigs::Side;
use malo_animations::{
	animations::{
		DorsoventralUndulation, FixedTuck, Flapping, FlipDirection, Gallop, Jab,
		LateralUndulation, Leap, QuadrupedLeap, QuadrupedRun, Run, Soaring, Tuck, TuckProfile,
		TuckedFlip, TwoFootedJump, TwoFootedTuckedFlip, UprightLeap, Walk, DEFAULT_BACKSWING,
		DEFAULT_GRAVITY,
		DEFAULT_JAB_TARGET, DEFAULT_JUMP_HEIGHT, DEFAULT_LANDING_SQUAT_SPEED,
		DEFAULT_PRE_SQUAT_SPEED,
	},
	Animation, Effects,
};

use crate::concepts::ConceptAnimation;
use crate::rig::{bone_map_ready, BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};

const RUN_CYCLE_SPEED: f32 = 1.4;
const WALK_CYCLE_SPEED: f32 = 0.9;
const GALLOP_CYCLE_SPEED: f32 = 0.35;
const QUADRUPED_RUN_CYCLE_SPEED: f32 = 0.5;
const TUCK_CYCLE_SPEED: f32 = 0.6;
const FRONT_FLIP_CYCLE_SPEED: f32 = 0.85;
const JAB_CYCLE_SPEED: f32 = 0.9;
const JUMP_PRE_SQUAT_SPEED: f32 = DEFAULT_PRE_SQUAT_SPEED * 1.2;
const JUMP_LANDING_SQUAT_SPEED: f32 = DEFAULT_LANDING_SQUAT_SPEED * 1.3;
/// One-shot leap lasts ~1.25 s so it covers the physics hang time.
const LEAP_CYCLE_SPEED: f32 = 0.8;
const BLEND_DURATION: f32 = 0.15;

/// Clip discriminant. Mailbox transitions key on this, not knob values.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum AnimId {
	#[default]
	Still,
	Walk,
	Run,
	QuadrupedRun,
	Gallop,
	Jump,
	Leap,
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
			Self::QuadrupedRun => QUADRUPED_RUN_CYCLE_SPEED,
			Self::Gallop => GALLOP_CYCLE_SPEED,
			Self::Jump => 1.0,
			Self::Leap => LEAP_CYCLE_SPEED,
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
		AnimClip::from(value).id()
	}
}

/// Untyped two-footed jump knobs ([`TwoFootedJump`] is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JumpParams {
	pub gravity: f32,
	pub jump_height: f32,
	pub pre_squat_speed: f32,
	pub landing_squat_speed: f32,
}

impl Default for JumpParams {
	fn default() -> Self {
		Self {
			gravity: DEFAULT_GRAVITY,
			jump_height: DEFAULT_JUMP_HEIGHT,
			pre_squat_speed: JUMP_PRE_SQUAT_SPEED,
			landing_squat_speed: JUMP_LANDING_SQUAT_SPEED,
		}
	}
}

impl JumpParams {
	fn apply_humanoid(self) -> TwoFootedJump<HumanoidV0Rig> {
		TwoFootedJump::default()
			.with_gravity(self.gravity)
			.with_jump_height(self.jump_height)
			.with_pre_squat_speed(self.pre_squat_speed)
			.with_landing_squat_speed(self.landing_squat_speed)
	}
}

/// Untyped tuck knobs ([`Tuck`] is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuckParams {
	pub tightness: f32,
}

impl Default for TuckParams {
	fn default() -> Self {
		Self { tightness: TuckProfile::DEFAULT_TIGHTNESS }
	}
}

/// Untyped tucked-flip knobs ([`TuckedFlip`] is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TuckedFlipParams {
	pub turns: f32,
	pub direction: FlipDirection,
	pub tightness: f32,
}

impl Default for TuckedFlipParams {
	fn default() -> Self {
		Self {
			turns: 1.0,
			direction: FlipDirection::Forward,
			tightness: TuckProfile::DEFAULT_TIGHTNESS,
		}
	}
}

impl TuckedFlipParams {
	fn apply_humanoid(self) -> TuckedFlip<HumanoidV0Rig> {
		let mut flip = TuckedFlip::default();
		flip.turns = self.turns;
		flip.direction = self.direction;
		flip.tuck = FixedTuck::new(self.tightness);
		flip
	}
}

/// Jump + flip bags for [`TwoFootedTuckedFlip`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct TwoFootedTuckedFlipParams {
	pub jump: JumpParams,
	pub flip: TuckedFlipParams,
}

/// Untyped jab knobs ([`Jab`] is rig-generic).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JabParams {
	pub side: Side,
	pub backswing: f32,
	pub target: bevy::prelude::Vec3,
}

impl Default for JabParams {
	fn default() -> Self {
		Self { side: Side::Right, backswing: DEFAULT_BACKSWING, target: DEFAULT_JAB_TARGET }
	}
}

/// Clip identity: variant + sampler knobs. Mailbox transitions use [`Self::id`].
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum AnimClip {
	#[default]
	Still,
	Walk(Walk),
	Run(Run),
	QuadrupedRun(QuadrupedRun),
	Gallop(Gallop),
	Jump(JumpParams),
	Leap(Leap),
	Tuck(TuckParams),
	TuckedFlip(TuckedFlipParams),
	TwoFootedTuckedFlip(TwoFootedTuckedFlipParams),
	Soaring(Soaring),
	Flapping(Flapping),
	Jab(JabParams),
	LateralUndulation(LateralUndulation),
	DorsoventralUndulation(DorsoventralUndulation),
}

impl AnimClip {
	pub const fn id(self) -> AnimId {
		match self {
			Self::Still => AnimId::Still,
			Self::Walk(_) => AnimId::Walk,
			Self::Run(_) => AnimId::Run,
			Self::QuadrupedRun(_) => AnimId::QuadrupedRun,
			Self::Gallop(_) => AnimId::Gallop,
			Self::Jump(_) => AnimId::Jump,
			Self::Leap(_) => AnimId::Leap,
			Self::Tuck(_) => AnimId::Tuck,
			Self::TuckedFlip(_) => AnimId::TuckedFlip,
			Self::TwoFootedTuckedFlip(_) => AnimId::TwoFootedTuckedFlip,
			Self::Soaring(_) => AnimId::Soaring,
			Self::Flapping(_) => AnimId::Flapping,
			Self::Jab(_) => AnimId::Jab,
			Self::LateralUndulation(_) => AnimId::LateralUndulation,
			Self::DorsoventralUndulation(_) => AnimId::DorsoventralUndulation,
		}
	}

	pub const fn default_speed(self) -> f32 {
		self.id().default_speed()
	}

	pub fn still() -> Self {
		Self::Still
	}

	pub fn walk() -> Self {
		Self::Walk(Walk::default())
	}

	pub fn run() -> Self {
		Self::Run(Run::default())
	}

	pub fn quadruped_run() -> Self {
		Self::QuadrupedRun(QuadrupedRun::default())
	}

	pub fn gallop() -> Self {
		Self::Gallop(Gallop::default())
	}

	pub fn jump() -> Self {
		Self::Jump(JumpParams::default())
	}

	pub fn leap() -> Self {
		Self::Leap(Leap::default())
	}

	pub fn tuck() -> Self {
		Self::Tuck(TuckParams::default())
	}

	pub fn tucked_flip() -> Self {
		Self::TuckedFlip(TuckedFlipParams::default())
	}

	pub fn two_footed_tucked_flip() -> Self {
		Self::TwoFootedTuckedFlip(TwoFootedTuckedFlipParams::default())
	}

	pub fn soaring() -> Self {
		Self::Soaring(Soaring::default())
	}

	pub fn flapping() -> Self {
		Self::Flapping(Flapping::default())
	}

	pub fn jab() -> Self {
		Self::Jab(JabParams::default())
	}

	pub fn lateral_undulation() -> Self {
		Self::LateralUndulation(LateralUndulation::default())
	}

	pub fn dorsoventral_undulation() -> Self {
		Self::DorsoventralUndulation(DorsoventralUndulation::default())
	}
}

impl From<ConceptAnimation> for AnimClip {
	fn from(value: ConceptAnimation) -> Self {
		match value {
			ConceptAnimation::Still => Self::still(),
			ConceptAnimation::Walk => Self::walk(),
			ConceptAnimation::Run => Self::run(),
			ConceptAnimation::Gallop => Self::gallop(),
			ConceptAnimation::Jump => Self::jump(),
			ConceptAnimation::Leap => Self::leap(),
			ConceptAnimation::Tuck => Self::tuck(),
			ConceptAnimation::TuckedFlip => Self::tucked_flip(),
			ConceptAnimation::TwoFootedTuckedFlip => Self::two_footed_tucked_flip(),
			ConceptAnimation::Soaring => Self::soaring(),
			ConceptAnimation::Flapping => Self::flapping(),
			ConceptAnimation::Jab => Self::jab(),
			ConceptAnimation::LateralUndulation => Self::lateral_undulation(),
			ConceptAnimation::DorsoventralUndulation => Self::dorsoventral_undulation(),
		}
	}
}

/// Clip + playback speed on a rig member.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AnimRef {
	pub clip: AnimClip,
	pub speed: f32,
}

impl AnimRef {
	pub fn new(clip: AnimClip) -> Self {
		Self { clip, speed: clip.default_speed() }
	}

	pub fn still() -> Self {
		Self::new(AnimClip::still())
	}
}

impl From<ConceptAnimation> for AnimRef {
	fn from(value: ConceptAnimation) -> Self {
		Self::new(AnimClip::from(value))
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
		let requested_id = requested.id();
		if mailbox.last != Some(requested_id) {
			mailbox.from_pose = if mailbox.output.is_empty() {
				rest_pose(bone_map, &bones)
			} else {
				mailbox.output.clone()
			};
			mailbox.blend_progress = 0.0;
			mailbox.clip_progress = 0.0;
			mailbox.last = Some(requested_id);
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

fn sample_humanoid(clip: AnimClip, rig: &mut HumanoidV0Rig, progress: f32) -> Effects {
	match clip {
		AnimClip::Still => Effects::default(),
		AnimClip::Walk(walk) => walk.apply(rig, progress),
		AnimClip::Run(run) => run.apply(rig, progress),
		AnimClip::Jump(params) => params.apply_humanoid().apply(rig, progress),
		AnimClip::Leap(leap) => UprightLeap::from_leap(&leap).apply(rig, progress),
		AnimClip::Tuck(params) => {
			Tuck::<HumanoidV0Rig>::new(params.tightness).apply(rig, progress.rem_euclid(1.0))
		}
		AnimClip::TuckedFlip(params) => {
			params.apply_humanoid().apply(rig, progress.rem_euclid(1.0))
		}
		AnimClip::TwoFootedTuckedFlip(params) => TwoFootedTuckedFlip::default()
			.with_jump(params.jump.apply_humanoid())
			.with_flip(params.flip.apply_humanoid())
			.apply(rig, progress),
		AnimClip::Soaring(soaring) => soaring.apply(rig, progress),
		AnimClip::Flapping(flapping) => flapping.apply(rig, progress),
		AnimClip::Jab(params) => Jab::<HumanoidV0Rig>::new(params.side, params.backswing, params.target)
			.apply(rig, progress.rem_euclid(1.0)),
		AnimClip::Gallop(_)
		| AnimClip::QuadrupedRun(_)
		| AnimClip::LateralUndulation(_)
		| AnimClip::DorsoventralUndulation(_) => Effects::default(),
	}
}

fn sample_quadruped(clip: AnimClip, rig: &mut QuadrupedV0Rig, progress: f32) -> Effects {
	match clip {
		AnimClip::QuadrupedRun(run) => run.apply(rig, progress),
		AnimClip::Run(_) => QuadrupedRun::default().apply(rig, progress),
		AnimClip::Gallop(gallop) => gallop.apply(rig, progress),
		AnimClip::Leap(leap) => QuadrupedLeap::from_leap(&leap).apply(rig, progress),
		_ => Effects::default(),
	}
}

fn sample_forelimbed(clip: AnimClip, rig: &mut ForelimbedV0Rig, progress: f32) -> Effects {
	match clip {
		AnimClip::LateralUndulation(wave) => wave.apply(rig, progress),
		AnimClip::DorsoventralUndulation(wave) => wave.apply(rig, progress),
		_ => Effects::default(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn concept_animation_maps_to_anim_ref() {
		let walk = AnimRef::from(ConceptAnimation::Walk);
		assert_eq!(walk.clip.id(), AnimId::Walk);
		assert_eq!(walk.clip, AnimClip::walk());
		assert_eq!(walk.speed, WALK_CYCLE_SPEED);
	}

	#[test]
	fn knob_changes_keep_clip_id() {
		let a = AnimClip::Walk(Walk { stride: 0.2, bounce: 1.0, rotation: 1.0 });
		let b = AnimClip::Walk(Walk { stride: 0.8, bounce: 2.0, rotation: 0.5 });
		assert_eq!(a.id(), b.id());
		assert_ne!(a, b);
		assert_ne!(AnimClip::walk().id(), AnimClip::run().id());
	}

	#[test]
	fn leap_is_a_distinct_clip() {
		let leap = AnimRef::from(ConceptAnimation::Leap);
		assert_eq!(leap.clip.id(), AnimId::Leap);
		assert_eq!(leap.clip, AnimClip::leap());
		assert_eq!(leap.speed, LEAP_CYCLE_SPEED);
		assert_ne!(AnimClip::leap().id(), AnimClip::jump().id());
	}
}
