//! Latest-wins mailbox that transitions from the last applied [`RigPose`].
//!
//! [`tick_anim_mailbox`] advances clip time for every body host. [`apply_anim_mailbox`]
//! samples and writes only hosts with [`AnimateBones`] and/or [`AnimateEffects`].
//! [`select_mailbox_applies`] rank-fills a count budget among Near bodies; leftovers
//! hold the last pose.

use std::collections::HashSet;

use bevy::ecs::batching::BatchingStrategy;
use bevy::ecs::query::{Has, Or};
use bevy::prelude::*;
use crozon_rigs::{
	forelimbed::ForelimbedRig,
	rigs::{
		forelimbed_v0::ForelimbedV0Rig, humanoid_v0::HumanoidV0Rig, quadruped_v0::QuadrupedV0Rig,
	},
	BonePose, Name as RigName, RigPose,
};
use intelligence_lod::{IntelligenceLod, IntelligencePriority};
use malo_animations::{
	animations::{
		Idle, Jab, QuadrupedIdle, QuadrupedLeap, QuadrupedRun, Tuck, TwoFootedTuckedFlip,
		UprightLeap,
	},
	Animation, Effects,
};

use crate::clip::{AnimClip, AnimId, AnimRefRoot};
use crate::markers::{AnimateBones, AnimateEffects, SuspendAnimation};
use crate::plant::plant_lod_entity;
use crate::rig::{bone_map_ready, BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};
use rigs::PoseSkipRotation;

/// Per-frame Near mailbox budget. Rank fills first; leftovers hold pose.
///
/// [`Self::max_applies`] is the NPC count. Missing-lod (local player) is extra
/// and always applies.
#[derive(Resource, Clone, Copy, Debug)]
pub struct MailboxApplyLimits {
	pub max_applies: usize,
	pub fairness_cap: u8,
}

impl Default for MailboxApplyLimits {
	fn default() -> Self {
		Self { max_applies: 32, fairness_cap: IntelligenceLod::FAIRNESS_CAP }
	}
}

/// Who may tick+apply this frame. `only = None` means every marked body (tests).
#[derive(Resource, Clone, Debug, Default)]
pub struct MailboxApplySet {
	pub only: Option<HashSet<Entity>>,
}

impl MailboxApplySet {
	fn allows(&self, entity: Entity) -> bool {
		self.only.as_ref().is_none_or(|only| only.contains(&entity))
	}
}

fn allows_only(only: Option<&HashSet<Entity>>, entity: Entity) -> bool {
	only.is_none_or(|only| only.contains(&entity))
}

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
	/// Frames this Near body was skipped. Reset when it applies.
	pub apply_skips: u8,
	last: Option<AnimId>,
	clip_progress: f32,
	blend_progress: f32,
	from_pose: RigPose,
	bind_transform: Transform,
}

/// Sample coordinate for the current clip. When present, [`tick_anim_mailbox`]
/// uses this instead of advancing wall-clock clip time (jump / leap phases).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AnimProgress(pub f32);

impl AnimMailbox {
	fn new(bind_transform: Transform) -> Self {
		Self {
			output: RigPose::new(),
			apply_skips: 0,
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
				.insert((AnimBone { name: name.clone(), rest: *bone_tf }, PoseSkipRotation));
		}

		commands.entity(entity).insert(AnimMailbox::new(*transform));
	}
}

/// Rank Near bodies, fill a count budget from the top, hold last pose on leftovers.
pub fn select_mailbox_applies(
	mut set: ResMut<MailboxApplySet>,
	limits: Res<MailboxApplyLimits>,
	priority: Res<IntelligencePriority>,
	mut hosts: Query<
		(Entity, &mut AnimMailbox, &CharacterRig),
		(
			With<AnimMailbox>,
			Without<AnimBone>,
			Without<SuspendAnimation>,
			Or<(With<AnimateBones>, With<AnimateEffects>)>,
		),
	>,
	child_of: Query<&ChildOf>,
	lods: Query<&IntelligenceLod>,
) {
	struct Candidate {
		entity: Entity,
		always: bool,
		rank: u32,
		skips: u8,
	}

	let mut candidates = Vec::new();
	for (entity, mailbox, character_rig) in &hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		let plant = plant_lod_entity(entity, &child_of, &lods);
		candidates.push(Candidate {
			entity,
			always: plant.is_none(),
			rank: plant.map(|plant| priority.rank_of(plant)).unwrap_or(0),
			skips: mailbox.apply_skips,
		});
	}
	candidates
		.sort_by_key(|candidate| (!candidate.always, candidate.rank, candidate.entity.to_bits()));

	let mut selected = HashSet::new();
	for candidate in candidates.iter().filter(|candidate| candidate.always) {
		selected.insert(candidate.entity);
	}
	let mut filled = 0;
	for candidate in candidates.iter().filter(|candidate| !candidate.always) {
		if filled >= limits.max_applies {
			break;
		}
		selected.insert(candidate.entity);
		filled += 1;
	}
	if let Some(fair) = candidates.iter().find(|candidate| {
		!selected.contains(&candidate.entity) && candidate.skips >= limits.fairness_cap
	}) {
		selected.insert(fair.entity);
	}

	for (entity, mut mailbox, character_rig) in &mut hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if selected.contains(&entity) {
			mailbox.apply_skips = 0;
		} else {
			mailbox.apply_skips = mailbox.apply_skips.saturating_add(1);
		}
	}
	set.only = Some(selected);
}

/// Advance clip / blend time for body mailboxes still owned by animation.
pub fn tick_anim_mailbox(
	time: Res<Time>,
	set: Option<Res<MailboxApplySet>>,
	mut hosts: Query<
		(Entity, &AnimRefRoot, &mut AnimMailbox, Option<&AnimProgress>, &CharacterRig, &BoneMap),
		(With<AnimMailbox>, Without<AnimBone>, Without<SuspendAnimation>),
	>,
	bones: Query<&AnimBone, Without<AnimMailbox>>,
) {
	let dt = time.delta_secs();
	for (entity, root, mut mailbox, progress, character_rig, bone_map) in &mut hosts {
		if !set.as_deref().is_none_or(|set| set.allows(entity)) {
			continue;
		}
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

		if let Some(progress) = progress {
			mailbox.clip_progress = progress.0;
		} else {
			mailbox.clip_progress += dt * root.0.speed;
		}
		if mailbox.blending() {
			mailbox.blend_progress = (mailbox.blend_progress + dt / BLEND_DURATION).min(1.0);
		}
	}
}

/// Sample clips in parallel; write bone transforms serially (shared bone query).
pub fn apply_anim_mailbox(
	set: Option<Res<MailboxApplySet>>,
	mut hosts: Query<
		(
			Entity,
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
		(
			With<AnimMailbox>,
			Without<AnimBone>,
			Without<SuspendAnimation>,
			Or<(With<AnimateBones>, With<AnimateEffects>)>,
		),
	>,
	bones: Query<&AnimBone, Without<AnimMailbox>>,
	mut bone_tfs: Query<&mut Transform, (With<AnimBone>, Without<AnimMailbox>)>,
) {
	let only = set.as_ref().and_then(|set| set.only.clone());
	hosts.par_iter_mut().batching_strategy(BatchingStrategy::fixed(8)).for_each(
		|(
			entity,
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
		)| {
			if !allows_only(only.as_ref(), entity) {
				return;
			}
			if character_rig.role != CharacterRigRole::Body {
				return;
			}

			let requested = root.0.clip;
			let rest = rest_pose(bone_map, &bones);
			let Some((sampled, effects)) = sample_requested(
				character_rig.skeleton,
				requested,
				&rest,
				clip_progress(requested, mailbox.clip_progress, entity),
				write_bones,
				write_effects,
				humanoid,
				quadruped,
				forelimbed,
			) else {
				return;
			};

			let weight = smoothstep(mailbox.blend_progress);
			if write_bones {
				mailbox.output = if mailbox.blending() || weight < 1.0 {
					RigPose::blend(&mailbox.from_pose, &sampled, weight)
				} else {
					sampled
				};
			}
			if write_effects {
				apply_root_motion(&mut armature, mailbox.bind_transform, effects, weight);
			}
		},
	);

	for (entity, _, mailbox, bone_map, character_rig, _, write_bones, _, _, _, _) in &hosts {
		if !write_bones || character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if !allows_only(only.as_ref(), entity) {
			continue;
		}
		write_pose(&mailbox.output, bone_map, &bones, &mut bone_tfs);
	}
}

fn clip_progress(clip: AnimClip, clip_progress: f32, entity: Entity) -> f32 {
	match clip {
		AnimClip::Still => clip_progress + Idle::phase_from_entity_bits(entity.to_bits()),
		_ => clip_progress,
	}
}

fn sample_requested(
	skeleton: RigSkeletonKind,
	requested: AnimClip,
	rest: &RigPose,
	clip_progress: f32,
	write_bones: bool,
	write_effects: bool,
	humanoid: Option<Mut<HumanoidV0Rig>>,
	quadruped: Option<Mut<QuadrupedV0Rig>>,
	forelimbed: Option<Mut<ForelimbedV0Rig>>,
) -> Option<(RigPose, Effects)> {
	Some(match skeleton {
		RigSkeletonKind::Humanoid => {
			let mut rig = humanoid?;
			seed_rig(&mut rig.pose, rest);
			let effects =
				sample_humanoid(requested, &mut rig, clip_progress, write_bones, write_effects);
			(rig.pose.clone(), effects)
		}
		RigSkeletonKind::Quadruped => {
			let mut rig = quadruped?;
			seed_rig(&mut rig.pose, rest);
			let effects =
				sample_quadruped(requested, &mut rig, clip_progress, write_bones, write_effects);
			(rig.pose.clone(), effects)
		}
		RigSkeletonKind::Forelimbed => {
			let mut rig = forelimbed?;
			seed_rig(&mut rig.pose, rest);
			let effects =
				sample_forelimbed(requested, &mut rig, clip_progress, write_bones, write_effects);
			(rig.pose.clone(), effects)
		}
		RigSkeletonKind::Neck => return None,
	})
}

fn rest_pose(bone_map: &BoneMap, bones: &Query<&AnimBone, Without<AnimMailbox>>) -> RigPose {
	let mut pose = RigPose::new();
	for entity in bone_map.by_name.values() {
		let Ok(bone) = bones.get(*entity) else {
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
	bones: &Query<&AnimBone, Without<AnimMailbox>>,
	transforms: &mut Query<&mut Transform, (With<AnimBone>, Without<AnimMailbox>)>,
) {
	for (name, entity) in &bone_map.by_name {
		let Ok(anim_bone) = bones.get(*entity) else {
			continue;
		};
		let Ok(mut transform) = transforms.get_mut(*entity) else {
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
		AnimClip::Still => {
			sample_split(&Idle::default(), rig, progress, write_bones, write_effects)
		}
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
		AnimClip::Still => {
			sample_split(&QuadrupedIdle::default(), rig, progress, write_bones, write_effects)
		}
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

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use intelligence_lod::IntelligenceBand;

	fn run_select(world: &mut World) {
		world.init_resource::<MailboxApplySet>();
		world.init_resource::<MailboxApplyLimits>();
		world.init_resource::<IntelligencePriority>();
		world.run_system_once(select_mailbox_applies).expect("select_mailbox_applies");
	}

	fn near_body(world: &mut World, parent: Entity) -> Entity {
		world
			.spawn((
				CharacterRig { role: CharacterRigRole::Body, ..default() },
				AnimMailbox::new(Transform::default()),
				AnimateBones,
				ChildOf(parent),
			))
			.id()
	}

	#[test]
	fn player_and_top_rank_apply_when_the_budget_is_one() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits {
			max_applies: 1,
			fairness_cap: IntelligenceLod::FAIRNESS_CAP,
		});
		let player_plant = world.spawn_empty().id();
		let top_plant = world.spawn(IntelligenceLod::missing()).id();
		let leftover_plant = world.spawn(IntelligenceLod::missing()).id();
		let player = near_body(&mut world, player_plant);
		let top = near_body(&mut world, top_plant);
		let leftover = near_body(&mut world, leftover_plant);
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(top_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(leftover_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&player));
		assert!(only.contains(&top));
		assert!(!only.contains(&leftover));
		assert_eq!(world.get::<AnimMailbox>(player).unwrap().apply_skips, 0);
		assert_eq!(world.get::<AnimMailbox>(top).unwrap().apply_skips, 0);
		assert_eq!(world.get::<AnimMailbox>(leftover).unwrap().apply_skips, 1);
	}

	#[test]
	fn leftover_applies_after_fairness_cap() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits { max_applies: 1, fairness_cap: 2 });
		let top_plant = world.spawn(IntelligenceLod::missing()).id();
		let leftover_plant =
			world.spawn(IntelligenceLod { band: IntelligenceBand::Near, skips: 0 }).id();
		let top = near_body(&mut world, top_plant);
		let leftover = near_body(&mut world, leftover_plant);
		world.get_mut::<AnimMailbox>(leftover).unwrap().apply_skips = 2;
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(top_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(leftover_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&top));
		assert!(only.contains(&leftover));
		assert_eq!(world.get::<AnimMailbox>(leftover).unwrap().apply_skips, 0);
	}

	#[test]
	fn still_samples_idle_on_humanoid() {
		let mut rig = HumanoidV0Rig::imported();
		for bone in ["shoulder.L", "shoulder.R", "humerus.L", "forearm.L", "lower_neck", "pelvis.L"]
		{
			rig.pose.insert(BonePose::new(RigName::from(bone), Transform::IDENTITY));
		}

		let effects = sample_humanoid(AnimClip::Still, &mut rig, 0.25, true, true);
		assert!(effects.r#move.is_none());
		let left = rig.pose.get(&RigName::from("shoulder.L")).expect("left");
		assert!(left.swing.abs() > 0.0);
		assert!(left.swing.abs() < 0.1);
		let humerus = rig.pose.get(&RigName::from("humerus.L")).expect("humerus");
		assert!(humerus.flex.abs() > 1.0);
	}

	#[test]
	fn still_samples_idle_on_quadruped() {
		let mut rig = QuadrupedV0Rig::imported();
		let effects =
			sample_quadruped(AnimClip::Still, &mut rig, QuadrupedIdle::graze_peak(), true, true);
		assert!(effects.r#move.is_none());
		let neck = rig.pose.get(&RigName::from("neck")).expect("neck");
		assert!(neck.swing < -0.5);
		let lumbar = rig.pose.get(&RigName::from("lumbar")).expect("lumbar");
		assert!(lumbar.flex > 0.1);
	}

	#[test]
	fn still_progress_offsets_by_entity_bits() {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		assert_ne!(clip_progress(AnimClip::Still, 0.0, a), clip_progress(AnimClip::Still, 0.0, b));
		assert_eq!(clip_progress(AnimClip::walk(), 0.3, a), 0.3);
	}

	#[test]
	fn count_budget_keeps_the_top_thirty_two() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits {
			max_applies: 32,
			fairness_cap: IntelligenceLod::FAIRNESS_CAP,
		});
		world.init_resource::<IntelligencePriority>();
		let mut bodies = Vec::new();
		for rank in 0..40u32 {
			let plant = world.spawn(IntelligenceLod::missing()).id();
			let body = near_body(&mut world, plant);
			world.resource_mut::<IntelligencePriority>().rank.insert(plant, rank);
			bodies.push((rank, body));
		}

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert_eq!(only.len(), 32);
		for (rank, body) in &bodies {
			if *rank < 32 {
				assert!(only.contains(body), "rank {rank} should apply");
			} else {
				assert!(!only.contains(body), "rank {rank} should hold pose");
			}
		}
	}
}
