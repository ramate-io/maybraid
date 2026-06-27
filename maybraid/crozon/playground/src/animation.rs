use bevy::prelude::*;
use clap::ValueEnum;
use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, BonePose, Name as RigName};
use malo_animations::{
	animations::{Run, Squat},
	Animation,
};

use crate::character::CharacterConfig;
use crate::skinning::{BoneMap, CharacterRig};

const RUN_CYCLE_SPEED: f32 = 0.5;
const SQUAT_CYCLE_SPEED: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum AnimationMode {
	#[default]
	Run,
	Squat,
}

#[derive(Component)]
pub struct LimbAnimator {
	pub bone: RigName,
	pub rest: Transform,
}

pub fn init_limb_animators(
	mut commands: Commands,
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

	commands.entity(rig_entity).insert(humanoid);
}

pub fn animate_limbs(
	config: Res<CharacterConfig>,
	mut rig: Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	mut limbs: Query<(Entity, &mut Transform, &LimbAnimator)>,
	time: Res<Time>,
) {
	match config.animation {
		AnimationMode::Run => animate_run(&mut rig, &mut limbs, time.elapsed_secs()),
		AnimationMode::Squat => animate_squat(&mut rig, &mut limbs, time.elapsed_secs()),
	}
}

fn animate_run(
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs);
	Run::<HumanoidV0Rig>::from_time(t, RUN_CYCLE_SPEED).apply(&mut rig);
	marshal_pose_to_limbs(&rig, limbs);
}

fn animate_squat(
	rig: &mut Query<&mut HumanoidV0Rig, With<CharacterRig>>,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
	t: f32,
) {
	let Ok(mut rig) = rig.single_mut() else {
		return;
	};

	marshal_limbs_into_pose(&mut rig, limbs);
	Squat::<HumanoidV0Rig>::from_time(t, SQUAT_CYCLE_SPEED).apply(&mut rig);
	marshal_pose_to_limbs(&rig, limbs);
}

/// Load each bone's rest transform into the rig pose as the animation starting point.
fn marshal_limbs_into_pose(
	rig: &mut HumanoidV0Rig,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
) {
	for (_, _, animator) in limbs.iter_mut() {
		rig.pose.insert(BonePose::new(animator.bone.clone(), animator.rest));
	}
}

/// Write the animated local transforms from the rig pose back onto the bone entities.
fn marshal_pose_to_limbs(
	rig: &HumanoidV0Rig,
	limbs: &mut Query<(Entity, &mut Transform, &LimbAnimator)>,
) {
	for (_, mut transform, animator) in limbs.iter_mut() {
		if let Some(pose) = rig.pose.get(&animator.bone) {
			*transform = pose.transform;
		}
	}
}
