//! Latest-wins mailbox that transitions from the last visible pose and armature offset.
//!
//! [`tick_anim_mailbox`] advances clip time for every body host. [`apply_anim_mailbox`]
//! samples and writes only hosts with [`AnimateBones`] and/or [`AnimateEffects`].
//! [`select_mailbox_applies`] rank-fills a count budget among on-screen marked
//! bodies; off-screen Near holds the last pose. Visible High Mid/Far plants
//! keep [`AnimateBones`] from sync, so they compete here too. No published look
//! means every marked body competes (tests / playgrounds).

use bevy::ecs::batching::BatchingStrategy;
use bevy::ecs::query::{Has, Or};
use bevy::prelude::*;
use character_animations::{
	animations::{
		Idle, Jab, Prone, ProneDescent, QuadrupedIdle, QuadrupedLeap, QuadrupedRun, Squat, Tuck,
		TwoFootedTuckedFlip, UprightLeap,
	},
	Animation, Effects,
};
use character_rigs::{
	authoring::{
		forelimbed_v0_definition, humanoid_v0_definition, quadruped_v0_definition, ArmatureOffset,
		BlendCurve, PoseBuffer, RigBinding, RigDefinition,
	},
	rigs::{
		forelimbed_v0::ForelimbedV0Rig, humanoid_v0::HumanoidV0Rig, quadruped_v0::QuadrupedV0Rig,
	},
	Name as RigName,
};
use intelligence_lod::{
	look_applies, IntelligenceFocus, IntelligenceLod, IntelligenceLookFrame, IntelligencePriority,
};

use crate::clip::{AnimClip, AnimId, AnimRefRoot};
use crate::markers::{AnimateBones, AnimateEffects, SuspendAnimation};
use crate::plant::plant_lod_entity;
use crate::rig::{bone_map_ready, BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};
use rigs::PoseSkipRotation;
use std::collections::HashSet;

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
	pub output: PoseBuffer,
	/// True after a bone sample has been written. Hold code waits for this.
	pub posed: bool,
	/// Offset currently shown on the armature. A new clip blends from here.
	pub displayed_offset: ArmatureOffset,
	/// Frames this Near body was skipped. Reset when it applies.
	pub apply_skips: u8,
	last: Option<AnimId>,
	clip_progress: f32,
	blend_progress: f32,
	from_pose: PoseBuffer,
	from_offset: ArmatureOffset,
	bind_transform: Transform,
}

/// Sample coordinate for the current clip. When present, [`tick_anim_mailbox`]
/// uses this instead of advancing wall-clock clip time (jump / leap phases).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct AnimProgress(pub f32);

/// Inserted when landmarks are ready but a required animation bone is still missing.
/// Prepare retries until the bone appears, and logs only on the first gap.
#[derive(Component, Clone, Copy, Default)]
pub struct AnimBindingGap;

impl AnimMailbox {
	pub fn new(bind_transform: Transform) -> Self {
		Self::with_bones(bind_transform, 0)
	}

	pub fn with_bones(bind_transform: Transform, len: usize) -> Self {
		Self {
			output: PoseBuffer::identity(len),
			posed: false,
			displayed_offset: ArmatureOffset::IDENTITY,
			apply_skips: 0,
			last: None,
			clip_progress: 0.0,
			blend_progress: 1.0,
			from_pose: PoseBuffer::identity(len),
			from_offset: ArmatureOffset::IDENTITY,
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
		(Entity, &AnimRefRoot, &BoneMap, &CharacterRig, &Transform, Has<AnimBindingGap>),
		(Without<AnimMailbox>, With<CharacterRig>),
	>,
	transforms: Query<&Transform>,
) {
	for (entity, _root, bone_map, character_rig, transform, gap) in &hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if !bone_map_ready(bone_map, character_rig.skeleton) {
			continue;
		}

		let definition = match character_rig.skeleton {
			RigSkeletonKind::Humanoid => humanoid_v0_definition(),
			RigSkeletonKind::Quadruped => quadruped_v0_definition(),
			RigSkeletonKind::Forelimbed => forelimbed_v0_definition(),
			RigSkeletonKind::Neck => continue,
		};
		let Some((rest, entities)) = capture_rest(&definition, bone_map, &transforms) else {
			if !gap {
				let missing = missing_animation_bones(&definition, bone_map);
				error!(
					"animation bones missing for {entity:?} ({:?}): {}",
					character_rig.skeleton,
					missing.join(", ")
				);
				commands.entity(entity).insert(AnimBindingGap);
			}
			continue;
		};
		if gap {
			commands.entity(entity).remove::<AnimBindingGap>();
		}

		let names: Vec<&'static str> = definition.names.iter().copied().collect();
		let binding =
			RigBinding::from_rest(definition, entities.clone().into_boxed_slice(), rest.clone());
		match character_rig.skeleton {
			RigSkeletonKind::Humanoid => {
				let mut rig = HumanoidV0Rig::imported();
				rig.binding = binding;
				rig.segment_lengths = rig.binding.metrics.humanoid_leg;
				rig.pose.copy_from(&rest);
				commands.entity(entity).insert(rig);
			}
			RigSkeletonKind::Quadruped => {
				let mut rig = QuadrupedV0Rig::imported();
				rig.binding = binding;
				rig.segment_lengths = rig.binding.metrics.quadruped_leg;
				rig.pose.copy_from(&rest);
				commands.entity(entity).insert(rig);
			}
			RigSkeletonKind::Forelimbed => {
				let mut rig = ForelimbedV0Rig::imported();
				rig.binding = binding;
				rig.pose.copy_from(&rest);
				commands.entity(entity).insert(rig);
			}
			RigSkeletonKind::Neck => continue,
		}

		for (entity_bone, name) in entities.iter().zip(names.iter().copied()) {
			let Ok(bone_tf) = transforms.get(*entity_bone) else {
				continue;
			};
			commands
				.entity(*entity_bone)
				.insert((AnimBone { name: RigName::from(name), rest: *bone_tf }, PoseSkipRotation));
		}

		let mut mailbox = AnimMailbox::with_bones(*transform, rest.len());
		mailbox.output.copy_from(&rest);
		mailbox.from_pose.copy_from(&rest);
		commands.entity(entity).insert(mailbox);
	}
}

/// Rank on-screen Near bodies, fill a count budget from the top, hold last pose
/// off-screen and on leftovers. Fairness only among in-cone leftovers.
pub fn select_mailbox_applies(
	mut set: ResMut<MailboxApplySet>,
	limits: Res<MailboxApplyLimits>,
	priority: Res<IntelligencePriority>,
	look_frame: Res<IntelligenceLookFrame>,
	focus: Res<IntelligenceFocus>,
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
	transforms: Query<&GlobalTransform>,
) {
	struct Candidate {
		entity: Entity,
		always: bool,
		in_cone: bool,
		rank: u32,
		skips: u8,
	}

	let mut candidates = Vec::new();
	for (entity, mailbox, character_rig) in &hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		let plant = plant_lod_entity(entity, &child_of, &lods);
		let always = plant.is_none();
		candidates.push(Candidate {
			entity,
			always,
			in_cone: always || in_apply_cone(plant, &look_frame, &focus, &transforms),
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
	for candidate in candidates.iter().filter(|candidate| !candidate.always && candidate.in_cone) {
		if filled >= limits.max_applies {
			break;
		}
		selected.insert(candidate.entity);
		filled += 1;
	}
	if let Some(fair) = candidates.iter().find(|candidate| {
		candidate.in_cone
			&& !selected.contains(&candidate.entity)
			&& candidate.skips >= limits.fairness_cap
	}) {
		selected.insert(fair.entity);
	}

	let in_cone: HashSet<Entity> = candidates
		.iter()
		.filter(|candidate| candidate.in_cone)
		.map(|candidate| candidate.entity)
		.collect();
	for (entity, mut mailbox, character_rig) in &mut hosts {
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}
		if selected.contains(&entity) {
			mailbox.apply_skips = 0;
		} else if in_cone.contains(&entity) {
			mailbox.apply_skips = mailbox.apply_skips.saturating_add(1);
		}
	}
	set.only = Some(selected);
}

fn in_apply_cone(
	plant: Option<Entity>,
	look_frame: &IntelligenceLookFrame,
	focus: &IntelligenceFocus,
	transforms: &Query<&GlobalTransform>,
) -> bool {
	if look_frame.look.is_none() && focus.samples.is_empty() {
		return true;
	}
	let Some(plant) = plant else {
		return true;
	};
	let Ok(transform) = transforms.get(plant) else {
		return true;
	};
	look_applies(transform.translation(), look_frame.look, focus)
}

/// Advance clip / blend time for body mailboxes still owned by animation.
pub fn tick_anim_mailbox(
	time: Res<Time>,
	set: Option<Res<MailboxApplySet>>,
	mut hosts: Query<
		(Entity, &AnimRefRoot, &mut AnimMailbox, Option<&AnimProgress>, &CharacterRig),
		(With<AnimMailbox>, Without<AnimBone>, Without<SuspendAnimation>),
	>,
) {
	let dt = time.delta_secs();
	for (entity, root, mut mailbox, progress, character_rig) in &mut hosts {
		if !set.as_deref().is_none_or(|set| set.allows(entity)) {
			continue;
		}
		if character_rig.role != CharacterRigRole::Body {
			continue;
		}

		let requested_id = root.0.clip.id();
		if mailbox.last != Some(requested_id) {
			if mailbox.posed {
				let AnimMailbox { output, from_pose, .. } = &mut *mailbox;
				from_pose.copy_from(output);
			}
			mailbox.from_offset = mailbox.displayed_offset;
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
			_bone_map,
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
			let progress = clip_progress(requested, mailbox.clip_progress, entity);
			let weight = BlendCurve::SmoothStep.sample(mailbox.blend_progress);
			let effects = match character_rig.skeleton {
				RigSkeletonKind::Humanoid => {
					let mut rig = match humanoid {
						Some(rig) => rig,
						None => return,
					};
					if write_bones {
						sync_humanoid_rest(&mut rig, &bones);
					}
					let effects =
						sample_humanoid(requested, &mut rig, progress, write_bones, write_effects);
					if write_bones {
						publish_pose(&mut mailbox, &rig.pose, weight);
					}
					effects
				}
				RigSkeletonKind::Quadruped => {
					let mut rig = match quadruped {
						Some(rig) => rig,
						None => return,
					};
					if write_bones {
						sync_quadruped_rest(&mut rig, &bones);
					}
					let effects =
						sample_quadruped(requested, &mut rig, progress, write_bones, write_effects);
					if write_bones {
						publish_pose(&mut mailbox, &rig.pose, weight);
					}
					effects
				}
				RigSkeletonKind::Forelimbed => {
					let mut rig = match forelimbed {
						Some(rig) => rig,
						None => return,
					};
					if write_bones {
						sync_binding_rest(&mut rig.binding, &bones);
					}
					let effects = sample_forelimbed(
						requested,
						&mut rig,
						progress,
						write_bones,
						write_effects,
					);
					if write_bones {
						publish_pose(&mut mailbox, &rig.pose, weight);
					}
					effects
				}
				RigSkeletonKind::Neck => return,
			};
			if write_effects {
				let shown = ArmatureOffset::blend(mailbox.from_offset, effects, weight);
				mailbox.displayed_offset = shown;
				*armature = shown.apply_to_bind(mailbox.bind_transform);
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
		let names = match character_rig.skeleton {
			RigSkeletonKind::Humanoid => humanoid_v0_definition(),
			RigSkeletonKind::Quadruped => quadruped_v0_definition(),
			RigSkeletonKind::Forelimbed => forelimbed_v0_definition(),
			RigSkeletonKind::Neck => continue,
		};
		write_pose(&mailbox.output, &names.names, bone_map, &mut bone_tfs);
	}
}

fn clip_progress(clip: AnimClip, clip_progress: f32, entity: Entity) -> f32 {
	match clip {
		AnimClip::Still => clip_progress + Idle::phase_from_entity_bits(entity.to_bits()),
		_ => clip_progress,
	}
}

fn publish_pose(mailbox: &mut AnimMailbox, sampled: &PoseBuffer, weight: f32) {
	if weight < 1.0 {
		PoseBuffer::blend_into(&mailbox.from_pose, sampled, weight, &mut mailbox.output);
	} else {
		mailbox.output.copy_from(sampled);
	}
	mailbox.posed = true;
}

fn sync_humanoid_rest(rig: &mut HumanoidV0Rig, bones: &Query<&AnimBone, Without<AnimMailbox>>) {
	if sync_binding_rest(&mut rig.binding, bones) {
		rig.segment_lengths = rig.binding.metrics.humanoid_leg;
	}
}

fn sync_quadruped_rest(rig: &mut QuadrupedV0Rig, bones: &Query<&AnimBone, Without<AnimMailbox>>) {
	if sync_binding_rest(&mut rig.binding, bones) {
		rig.segment_lengths = rig.binding.metrics.quadruped_leg;
	}
}

/// Refresh rest and segment lengths when a bone rest changed.
fn sync_binding_rest(
	binding: &mut RigBinding,
	bones: &Query<&AnimBone, Without<AnimMailbox>>,
) -> bool {
	if binding
		.entities
		.iter()
		.zip(binding.effective_rest.local.iter())
		.all(|(entity, rest)| bones.get(*entity).is_ok_and(|bone| bone.rest == *rest))
	{
		return false;
	}
	let Some(rest) = rest_from_bones(binding, bones) else {
		return false;
	};
	binding.refresh_rest(rest);
	true
}

fn rest_from_bones(
	binding: &RigBinding,
	bones: &Query<&AnimBone, Without<AnimMailbox>>,
) -> Option<PoseBuffer> {
	let mut rest = PoseBuffer::identity(binding.definition.len());
	for (index, entity) in binding.entities.iter().enumerate() {
		rest.local[index] = bones.get(*entity).ok()?.rest;
	}
	Some(rest)
}

fn capture_rest(
	definition: &RigDefinition,
	bone_map: &BoneMap,
	transforms: &Query<&Transform>,
) -> Option<(PoseBuffer, Vec<Entity>)> {
	let mut rest = PoseBuffer::identity(definition.len());
	let mut entities = vec![Entity::PLACEHOLDER; definition.len()];
	for (index, name) in definition.names.iter().enumerate() {
		let bone_entity = *bone_map.by_name.get(*name)?;
		rest.local[index] = *transforms.get(bone_entity).ok()?;
		entities[index] = bone_entity;
	}
	Some((rest, entities))
}

fn missing_animation_bones(definition: &RigDefinition, bone_map: &BoneMap) -> Vec<&'static str> {
	definition
		.names
		.iter()
		.copied()
		.filter(|name| !bone_map.by_name.contains_key(*name))
		.collect()
}

fn write_pose(
	pose: &PoseBuffer,
	names: &[&'static str],
	bone_map: &BoneMap,
	transforms: &mut Query<&mut Transform, (With<AnimBone>, Without<AnimMailbox>)>,
) {
	for (index, name) in names.iter().enumerate() {
		let Some(&entity) = bone_map.by_name.get(*name) else {
			continue;
		};
		let Some(desired) = pose.local.get(index) else {
			continue;
		};
		let Ok(mut transform) = transforms.get_mut(entity) else {
			continue;
		};
		if *transform != *desired {
			*transform = *desired;
		}
	}
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
		Effects::IDENTITY
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
			&Tuck::new(params.tightness),
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
			&Jab::new(params.side, params.backswing, params.target),
			rig,
			progress.rem_euclid(1.0),
			write_bones,
			write_effects,
		),
		AnimClip::Squat => sample_split(&Squat::held(), rig, progress, write_bones, write_effects),
		AnimClip::ProneDescent => {
			sample_split(&ProneDescent::default(), rig, progress, write_bones, write_effects)
		}
		AnimClip::Prone => {
			sample_split(&Prone::default(), rig, progress, write_bones, write_effects)
		}
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
		world.init_resource::<IntelligenceLookFrame>();
		world.init_resource::<IntelligenceFocus>();
		world.run_system_once(select_mailbox_applies).expect("select_mailbox_applies");
	}

	fn looking_x_apply() -> intelligence_lod::IntelligenceLook {
		let tf =
			Transform::from_translation(Vec3::Y).looking_at(Vec3::new(20.0, 1.0, 0.0), Vec3::Y);
		intelligence_lod::IntelligenceLook::from_perspective_inset(
			&GlobalTransform::from(tf),
			&PerspectiveProjection {
				fov: 75_f32.to_radians(),
				aspect_ratio: 16.0 / 9.0,
				..default()
			},
			75_f32.to_radians(),
			intelligence_lod::LOOK_APPLY_FOV_INSET,
		)
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
		let effects = sample_humanoid(AnimClip::Still, &mut rig, 0.25, true, true);
		assert!(effects.is_identity());
		assert!(rig.posed_angle("shoulder.L") > 0.0);
		assert!(rig.posed_angle("shoulder.L") < 0.15);
		assert!(rig.posed_angle("humerus.L") > 0.2, "Still should hang the arms");
	}

	#[test]
	fn stance_squat_has_no_root_move() -> anyhow::Result<()> {
		use anyhow::anyhow;

		let mut rig = HumanoidV0Rig::imported();
		let effects = sample_humanoid(AnimClip::squat(), &mut rig, 1.0, true, true);
		if !effects.is_identity() {
			return Err(anyhow!("held squat must not move the armature"));
		}
		if rig.posed_angle("femur.L") < 0.3 {
			return Err(anyhow!("held squat should flex femurs"));
		}
		if rig.posed_angle("pelvis.L") < 0.1 {
			return Err(anyhow!("held squat should crease the pelvis"));
		}
		Ok(())
	}

	#[test]
	fn stance_prone_pitches_the_spine() -> anyhow::Result<()> {
		use anyhow::anyhow;

		let mut rig = HumanoidV0Rig::imported();
		let effects = sample_humanoid(AnimClip::prone(), &mut rig, 1.0, true, true);
		if !effects.is_identity() {
			return Err(anyhow!("held prone must not move the armature"));
		}
		let root = rig.rotation("root") * Vec3::Y;
		if root.z.abs() < 0.2 || root.x.abs() > 0.2 {
			return Err(anyhow!("prone should pitch the spine, got {root:?}"));
		}
		Ok(())
	}

	#[test]
	fn still_samples_idle_on_quadruped() {
		let mut rig = QuadrupedV0Rig::imported();
		let effects =
			sample_quadruped(AnimClip::Still, &mut rig, QuadrupedIdle::graze_peak(), true, true);
		assert!(effects.is_identity());
		let neck = rig.rotation("neck") * Vec3::Y;
		assert!((neck - Vec3::Y).length() > 0.2, "neck leaves rest, got {neck:?}");
		let lumbar = rig.rotation("lumbar") * Vec3::Y;
		assert!((lumbar - Vec3::Y).length() > 0.05, "lumbar leaves rest, got {lumbar:?}");
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

	#[test]
	fn in_cone_near_applies_before_off_cone() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits {
			max_applies: 1,
			fairness_cap: IntelligenceLod::FAIRNESS_CAP,
		});
		world.insert_resource(IntelligenceLookFrame { look: Some(looking_x_apply()) });
		let ahead_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(40.0, 1.0, 0.0)),
			))
			.id();
		let behind_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(-40.0, 1.0, 0.0)),
			))
			.id();
		let ahead = near_body(&mut world, ahead_plant);
		let behind = near_body(&mut world, behind_plant);
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(behind_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(ahead_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&ahead));
		assert!(!only.contains(&behind));
		assert_eq!(world.get::<AnimMailbox>(behind).unwrap().apply_skips, 0);
	}

	#[test]
	fn off_cone_does_not_steal_fairness() {
		let mut world = World::new();
		world.insert_resource(MailboxApplyLimits { max_applies: 1, fairness_cap: 2 });
		world.insert_resource(IntelligenceLookFrame { look: Some(looking_x_apply()) });
		let ahead_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(40.0, 1.0, 0.0)),
			))
			.id();
		let behind_plant = world
			.spawn((
				IntelligenceLod::missing(),
				GlobalTransform::from_translation(Vec3::new(-40.0, 1.0, 0.0)),
			))
			.id();
		let ahead = near_body(&mut world, ahead_plant);
		let behind = near_body(&mut world, behind_plant);
		world.get_mut::<AnimMailbox>(behind).unwrap().apply_skips = 8;
		world.init_resource::<IntelligencePriority>();
		world.resource_mut::<IntelligencePriority>().rank.insert(ahead_plant, 0);
		world.resource_mut::<IntelligencePriority>().rank.insert(behind_plant, 1);

		run_select(&mut world);

		let only = world.resource::<MailboxApplySet>().only.clone().expect("restricted set");
		assert!(only.contains(&ahead));
		assert!(!only.contains(&behind));
		assert_eq!(world.get::<AnimMailbox>(behind).unwrap().apply_skips, 8);
	}
}
