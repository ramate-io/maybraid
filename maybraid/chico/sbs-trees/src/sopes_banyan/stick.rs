//! Stick segment → [`StickNode`] emission (with structural LOD phase filters).

use bevy::prelude::Vec3;
use chico_sbs_geometry::{BallStickSegment, SopesBanyanChain, SopesBanyanPhase};
use chico_vegetation_components::{Placement, StickNode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StickLodRole {
	/// Strict stalk / trunk.
	Trunk,
	/// Descender aerial roots.
	Descender,
	/// Branch-out / flair sticks that connect successive ball-stick nodes.
	BetweenNodes,
}

impl StickLodRole {
	pub(crate) fn from_parent_phase(phase: &SopesBanyanPhase) -> Self {
		match phase {
			SopesBanyanPhase::Stalk(_) => Self::Trunk,
			SopesBanyanPhase::StartDescender(_) | SopesBanyanPhase::EndDescender(_) => {
				Self::Descender
			}
			SopesBanyanPhase::BranchOut(_)
			| SopesBanyanPhase::StartFlairUp(_)
			| SopesBanyanPhase::EndFlairUp(_) => Self::BetweenNodes,
		}
	}

	pub(crate) fn keep_on_low(self) -> bool {
		matches!(self, Self::Trunk)
	}
}

pub(crate) fn stick_node_for_segment(segment: &BallStickSegment<'_>) -> Option<StickNode> {
	let ray = segment.ray();
	let len_sq = ray.length_squared();
	if len_sq < 1e-12 {
		return None;
	}
	let length = len_sq.sqrt();
	let placement =
		Placement::stick_segment(segment.start.position, ray, length, segment.start.radius)?;
	Some(StickNode::segment(placement))
}

pub(crate) fn stick_role_for_segment(
	_segment: &BallStickSegment<'_>,
	parent: &SopesBanyanChain,
) -> StickLodRole {
	StickLodRole::from_parent_phase(&parent.phase)
}

fn horizontal_radius(position: Vec3) -> f32 {
	Vec3::new(position.x, 0.0, position.z).length()
}

/// Midpoint footprint radius of a segment (for outer-half silhouette keep).
pub(crate) fn segment_horizontal_radius(segment: &BallStickSegment<'_>) -> f32 {
	let mid = (segment.start.position + segment.end.position) * 0.5;
	horizontal_radius(mid)
}

/// Medium: always keep trunk; keep other sticks only in the outer half of the footprint.
pub(crate) fn keep_stick_on_medium(
	role: StickLodRole,
	segment: &BallStickSegment<'_>,
	tree_radius: f32,
) -> bool {
	if matches!(role, StickLodRole::Trunk) {
		return true;
	}
	segment_horizontal_radius(segment) >= tree_radius.max(1e-4) * 0.5
}
