use std::sync::Arc;

use bevy::prelude::*;

use super::buffer::PoseBuffer;
use super::frame::JointFrame;
use crate::humanoid::LegSegmentLengths as HumanoidLengths;
use crate::quadruped::LegSegmentLengths as QuadrupedLengths;

/// Bone index inside one [`RigDefinition`]. Not comparable across definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoneId(pub u16);

impl BoneId {
	pub fn index(self) -> usize {
		self.0 as usize
	}
}

/// Which sampler and metric set a definition uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkeletonFamily {
	Humanoid,
	Quadruped,
	Forelimbed,
}

/// Shared, immutable skeleton: names, parents, and evaluation order.
///
/// Names are for import and diagnostics. Sampling indexes [`BoneId`].
#[derive(Debug, Clone)]
pub struct RigDefinition {
	pub family: SkeletonFamily,
	pub names: Box<[&'static str]>,
	pub parents: Box<[Option<BoneId>]>,
	pub evaluation_order: Box<[BoneId]>,
}

impl RigDefinition {
	pub fn from_names(
		family: SkeletonFamily,
		names: &[&'static str],
		parent_names: &[(&str, &str)],
	) -> Self {
		let parents = names
			.iter()
			.map(|name| {
				parent_names
					.iter()
					.find(|(child, parent)| child == name && !parent.is_empty())
					.and_then(|(_, parent)| {
						names
							.iter()
							.position(|candidate| candidate == parent)
							.map(|index| BoneId(index as u16))
					})
			})
			.collect::<Vec<_>>()
			.into_boxed_slice();
		let evaluation_order = evaluation_order(&parents);
		Self { family, names: names.to_vec().into_boxed_slice(), parents, evaluation_order }
	}

	pub fn len(&self) -> usize {
		self.names.len()
	}

	pub fn id(&self, name: &str) -> Option<BoneId> {
		self.names
			.iter()
			.position(|candidate| *candidate == name)
			.map(|index| BoneId(index as u16))
	}

	pub fn parent(&self, bone: BoneId) -> Option<BoneId> {
		self.parents.get(bone.index()).copied().flatten()
	}

	/// Character-space rotation of `bone` in `pose`, parents first.
	pub fn rotation_in_character(&self, pose: &PoseBuffer, bone: BoneId) -> Quat {
		let mut chain = [BoneId(0); 24];
		let mut count = 0;
		let mut cursor = Some(bone);
		while let Some(id) = cursor {
			if count >= chain.len() {
				break;
			}
			chain[count] = id;
			count += 1;
			cursor = self.parent(id);
		}
		let mut rotation = Quat::IDENTITY;
		for id in chain[..count].iter().rev() {
			rotation *= pose.rotation(*id);
		}
		rotation
	}

	pub fn parent_rotation(&self, pose: &PoseBuffer, bone: BoneId) -> Quat {
		match self.parent(bone) {
			Some(parent) => self.rotation_in_character(pose, parent),
			None => Quat::IDENTITY,
		}
	}
}

fn evaluation_order(parents: &[Option<BoneId>]) -> Box<[BoneId]> {
	let mut seen = vec![false; parents.len()];
	let mut order = Vec::with_capacity(parents.len());
	fn visit(index: usize, parents: &[Option<BoneId>], seen: &mut [bool], order: &mut Vec<BoneId>) {
		if seen.get(index).copied().unwrap_or(true) {
			return;
		}
		if let Some(parent) = parents[index] {
			visit(parent.index(), parents, seen, order);
		}
		seen[index] = true;
		order.push(BoneId(index as u16));
	}
	for index in 0..parents.len() {
		visit(index, parents, &mut seen, &mut order);
	}
	order.into_boxed_slice()
}

/// Per-character rest, entities, and calibrated frames.
///
/// `frames[i]` matches `definition` bone `i`. Rebuild frames only when rest changes.
#[derive(Clone, Debug)]
pub struct RigBinding {
	pub definition: Arc<RigDefinition>,
	pub entities: Box<[Entity]>,
	pub effective_rest: PoseBuffer,
	pub frames: Box<[JointFrame]>,
	pub metrics: RigMetrics,
}

impl RigBinding {
	pub fn from_rest(
		definition: Arc<RigDefinition>,
		entities: Box<[Entity]>,
		effective_rest: PoseBuffer,
	) -> Self {
		let frames = calibrate(&definition, &effective_rest);
		let metrics = RigMetrics::from_rest(&definition, &effective_rest);
		Self { definition, entities, effective_rest, frames, metrics }
	}

	/// Replace rest when a proportion edit or bone-map reload changes bind transforms.
	pub fn refresh_rest(&mut self, effective_rest: PoseBuffer) {
		if effective_rest.local == self.effective_rest.local {
			return;
		}
		self.frames = calibrate(&self.definition, &effective_rest);
		self.metrics = RigMetrics::from_rest(&self.definition, &effective_rest);
		self.effective_rest = effective_rest;
	}
}

/// Lengths derived from effective rest. Zero bind translations keep the family default.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigMetrics {
	pub humanoid_leg: HumanoidLengths,
	pub quadruped_leg: QuadrupedLengths,
}

impl Default for RigMetrics {
	fn default() -> Self {
		Self {
			humanoid_leg: HumanoidLengths::default(),
			quadruped_leg: QuadrupedLengths::default(),
		}
	}
}

impl RigMetrics {
	pub fn from_rest(definition: &RigDefinition, rest: &PoseBuffer) -> Self {
		let mut metrics = Self::default();
		let length = |name: &str| -> f32 {
			definition
				.id(name)
				.and_then(|id| rest.get(id))
				.map(|transform| transform.translation.length())
				.filter(|length| *length > 1e-3)
				.unwrap_or(0.0)
		};
		match definition.family {
			SkeletonFamily::Humanoid => {
				let femur = length("femur.L");
				let shin = length("shin.L");
				if femur > 0.0 {
					metrics.humanoid_leg.femur = femur;
				}
				if shin > 0.0 {
					metrics.humanoid_leg.shin = shin;
				}
			}
			SkeletonFamily::Quadruped => {
				let upper = length("anterior_thigh.L");
				let lower = length("anterior_shin.L");
				if upper > 0.0 {
					metrics.quadruped_leg.upper = upper;
				}
				if lower > 0.0 {
					metrics.quadruped_leg.lower = lower;
				}
			}
			SkeletonFamily::Forelimbed => {}
		}
		metrics
	}
}

fn calibrate(definition: &RigDefinition, rest: &PoseBuffer) -> Box<[JointFrame]> {
	let mut frames = vec![JointFrame::IDENTITY; definition.len()];
	for (index, frame) in frames.iter_mut().enumerate() {
		let bone = BoneId(index as u16);
		let rotation = rest.rotation(bone);
		*frame = match definition.family {
			SkeletonFamily::Humanoid => super::humanoid::frame_for(definition, bone, rotation),
			SkeletonFamily::Quadruped => super::quadruped::frame_for(definition, bone, rotation),
			SkeletonFamily::Forelimbed => super::forelimbed::frame_for(rotation),
		};
	}
	frames.into_boxed_slice()
}
