//! Latest-wins mailbox that transitions from the last applied [`RigPose`].
//!
//! [`tick_anim_mailbox`] advances clip time for every body host. [`apply_anim_mailbox`]
//! samples and writes only hosts with [`AnimateBones`] and/or [`AnimateEffects`].

use bevy::ecs::query::{Has, Or};
use bevy::prelude::*;
use crozon_rigs::{
	forelimbed::ForelimbedRig,
	rigs::{
		forelimbed_v0::ForelimbedV0Rig, humanoid_v0::HumanoidV0Rig, quadruped_v0::QuadrupedV0Rig,
	},
	BonePose, Name as RigName, RigPose,
};
use malo_animations::{
	animations::{Jab, QuadrupedLeap, QuadrupedRun, Tuck, TwoFootedTuckedFlip, UprightLeap},
	Animation, Effects,
};

use crate::clip::{AnimClip, AnimId, AnimRefRoot};
use crate::markers::{AnimateBones, AnimateEffects};
use crate::rig::{bone_map_ready, BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};

const BLEND_DURATION: f32 = 0.15;

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
///
/// Does not require [`AnimateBones`] — coming back into range should not rebuild
/// the mailbox.
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

/// Advance clip / blend time for every body mailbox (cheap; runs far from camera too).
pub fn tick_anim_mailbox(
	time: Res<Time>,
	mut hosts: Query<
		(&AnimRefRoot, &mut AnimMailbox, &CharacterRig, &BoneMap),
		(With<AnimMailbox>, Without<AnimBone>),
	>,
	bones: Query<(&AnimBone, &mut Transform), Without<AnimMailbox>>,
) {
	let dt = time.delta_secs();
	for (root, mut mailbox, character_rig, bone_map) in &mut hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}

		let requested_id = root.0.clip.id();
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
	}
}

/// Sample and write bones / root-motion only when the host carries those markers.
pub fn apply_anim_mailbox(
	mut hosts: Query<
		(
			&AnimRefRoot,
			&mut AnimMailbox,
			&BoneMap,
			&CharacterRig,
			&mut Transform,
			Has<AnimateBones>,
			Has<AnimateEffects>,
			Option<&mut HumanoidV0Rig>,
			Option<&mut QuadrupedV0Rig>,
			Option<&mut ForelimbedV0Rig>,
		),
		(With<AnimMailbox>, Without<AnimBone>, Or<(With<AnimateBones>, With<AnimateEffects>)>),
	>,
	mut bones: Query<(&AnimBone, &mut Transform), Without<AnimMailbox>>,
) {
	for (
		root,
		mut mailbox,
		bone_map,
		character_rig,
		mut armature,
		write_bones,
		write_effects,
		humanoid,
		quadruped,
		forelimbed,
	) in &mut hosts
	{
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}

		let requested = root.0.clip;
		let rest = rest_pose(bone_map, &bones);
		let (sampled, effects) = match character_rig.skeleton {
			RigSkeletonKind::Humanoid => {
				let Some(mut rig) = humanoid else {
					continue;
				};
				seed_rig(&mut rig.pose, &rest);
				let effects = sample_humanoid(
					requested,
					&mut rig,
					mailbox.clip_progress,
					write_bones,
					write_effects,
				);
				(rig.pose.clone(), effects)
			}
			RigSkeletonKind::Quadruped => {
				let Some(mut rig) = quadruped else {
					continue;
				};
				seed_rig(&mut rig.pose, &rest);
				let effects = sample_quadruped(
					requested,
					&mut rig,
					mailbox.clip_progress,
					write_bones,
					write_effects,
				);
				(rig.pose.clone(), effects)
			}
			RigSkeletonKind::Forelimbed => {
				let Some(mut rig) = forelimbed else {
					continue;
				};
				seed_rig(&mut rig.pose, &rest);
				let effects = sample_forelimbed(
					requested,
					&mut rig,
					mailbox.clip_progress,
					write_bones,
					write_effects,
				);
				(rig.pose.clone(), effects)
			}
			RigSkeletonKind::Neck => continue,
		};

		let weight = smoothstep(mailbox.blend_progress);
		if write_bones {
			let pose = if mailbox.blending() || weight < 1.0 {
				RigPose::blend(&mailbox.from_pose, &sampled, weight)
			} else {
				sampled
			};
			write_pose(&pose, bone_map, &mut bones);
			mailbox.output = pose;
		}
		if write_effects {
			apply_root_motion(&mut armature, mailbox.bind_transform, effects, weight);
		}
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

fn sample_split<A, R>(
	anim: &A,
	rig: &mut R,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects
where
	A: Animation<R>,
{
	if write_bones {
		anim.apply_for(rig, progress);
	}
	if write_effects {
		anim.effects_for(rig, progress)
	} else {
		Effects::default()
	}
}

fn sample_humanoid(
	clip: AnimClip,
	rig: &mut HumanoidV0Rig,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects {
	match clip {
		AnimClip::Still => Effects::default(),
		AnimClip::Walk(walk) => sample_split(&walk, rig, progress, write_bones, write_effects),
		AnimClip::Run(run) => sample_split(&run, rig, progress, write_bones, write_effects),
		AnimClip::Jump(params) => {
			sample_split(&params.apply_humanoid(), rig, progress, write_bones, write_effects)
		}
		AnimClip::Leap(leap) => {
			sample_split(&UprightLeap::from_leap(&leap), rig, progress, write_bones, write_effects)
		}
		AnimClip::Tuck(params) => sample_split(
			&Tuck::<HumanoidV0Rig>::new(params.tightness),
			rig,
			progress.rem_euclid(1.0),
			write_bones,
			write_effects,
		),
		AnimClip::TuckedFlip(params) => sample_split(
			&params.apply_humanoid(),
			rig,
			progress.rem_euclid(1.0),
			write_bones,
			write_effects,
		),
		AnimClip::TwoFootedTuckedFlip(params) => sample_split(
			&TwoFootedTuckedFlip::default()
				.with_jump(params.jump.apply_humanoid())
				.with_flip(params.flip.apply_humanoid()),
			rig,
			progress,
			write_bones,
			write_effects,
		),
		AnimClip::Soaring(soaring) => {
			sample_split(&soaring, rig, progress, write_bones, write_effects)
		}
		AnimClip::Flapping(flapping) => {
			sample_split(&flapping, rig, progress, write_bones, write_effects)
		}
		AnimClip::Jab(params) => sample_split(
			&Jab::<HumanoidV0Rig>::new(params.side, params.backswing, params.target),
			rig,
			progress.rem_euclid(1.0),
			write_bones,
			write_effects,
		),
		AnimClip::Gallop(_)
		| AnimClip::QuadrupedRun(_)
		| AnimClip::LateralUndulation(_)
		| AnimClip::DorsoventralUndulation(_) => Effects::default(),
	}
}

fn sample_quadruped(
	clip: AnimClip,
	rig: &mut QuadrupedV0Rig,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects {
	match clip {
		AnimClip::QuadrupedRun(run) => {
			sample_split(&run, rig, progress, write_bones, write_effects)
		}
		AnimClip::Run(_) => {
			sample_split(&QuadrupedRun::default(), rig, progress, write_bones, write_effects)
		}
		AnimClip::Gallop(gallop) => {
			sample_split(&gallop, rig, progress, write_bones, write_effects)
		}
		AnimClip::Leap(leap) => sample_split(
			&QuadrupedLeap::from_leap(&leap),
			rig,
			progress,
			write_bones,
			write_effects,
		),
		_ => Effects::default(),
	}
}

fn sample_forelimbed(
	clip: AnimClip,
	rig: &mut ForelimbedV0Rig,
	progress: f32,
	write_bones: bool,
	write_effects: bool,
) -> Effects {
	match clip {
		AnimClip::LateralUndulation(wave) => {
			sample_split(&wave, rig, progress, write_bones, write_effects)
		}
		AnimClip::DorsoventralUndulation(wave) => {
			sample_split(&wave, rig, progress, write_bones, write_effects)
		}
		_ => Effects::default(),
	}
}
