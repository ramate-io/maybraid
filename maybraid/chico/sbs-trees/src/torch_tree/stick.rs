//! Storybook / torch stick → [`StickNode`] emission (with structural LOD filters).

use bevy::prelude::Vec3;
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	BallStickSegment, StorybookTreeChain, StorybookTreePhase,
};
use chico_vegetation_components::{Placement, StickNode};

/// Medium sticks: coarser azimuth × height outer samples (aggressive drop-off).
pub(crate) const MEDIUM_STICK_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

/// Keep roughly this fraction of branch sticks on Low (stable every-Nth sample).
pub(crate) const LOW_BRANCH_KEEP_EVERY: usize = 4;

fn is_stalk(parent: &StorybookTreeChain) -> bool {
	matches!(parent.phase, StorybookTreePhase::Stalk(_))
}

pub(crate) fn stick_node_for_segment(
	segment: &BallStickSegment<'_>,
	parent: &StorybookTreeChain,
) -> Option<StickNode> {
	let start = segment.start.position;
	let end = segment.end.position;
	let radius = segment.start.radius;
	let ray = end - start;
	let len_sq = ray.length_squared();
	if len_sq < 1e-12 {
		return None;
	}
	let length = len_sq.sqrt();
	let placement = Placement::stick_segment(start, ray, length, radius)?;
	if is_stalk(parent) {
		Some(StickNode::trunk(placement))
	} else {
		Some(StickNode::segment(placement))
	}
}

pub(crate) fn stick_nodes_high(chain: &BallStickChain<StorybookTreeChain>) -> Vec<StickNode> {
	chain
		.segments_with_hysteresis()
		.filter_map(|(segment, parent, _)| stick_node_for_segment(&segment, parent))
		.collect()
}

#[derive(Clone, Copy)]
struct StickBandCandidate {
	mid: Vec3,
	start: Vec3,
	end: Vec3,
	radius: f32,
}

/// Stalk always + outermost non-stalk sticks per azimuth × height cell.
pub(crate) fn stick_nodes_medium_banded(
	chain: &BallStickChain<StorybookTreeChain>,
) -> Vec<StickNode> {
	let mut trunk = Vec::new();
	let mut candidates = Vec::new();
	for (segment, parent, _) in chain.segments_with_hysteresis() {
		if is_stalk(parent) {
			if let Some(node) = stick_node_for_segment(&segment, parent) {
				trunk.push(node);
			}
			continue;
		}
		candidates.push(StickBandCandidate {
			mid: segment.midpoint(),
			start: segment.start.position,
			end: segment.end.position,
			radius: segment.start.radius,
		});
	}
	let sampled = sample_max_horizontal_radius_by_azimuth_height(
		&candidates,
		|c| c.mid,
		MEDIUM_STICK_BANDS,
	);
	trunk.extend(sampled.into_iter().filter_map(|s| {
		StickNode::from_segment(s.item.start, s.item.end, s.item.radius)
	}));
	trunk
}

/// Low: stalk + a thinned subset of branch sticks.
pub(crate) fn keep_branch_on_low(is_stalk_seg: bool, branch_index: &mut usize) -> bool {
	if is_stalk_seg {
		return true;
	}
	let keep = *branch_index % LOW_BRANCH_KEEP_EVERY == 0;
	*branch_index += 1;
	keep
}

pub(crate) fn stick_nodes_low(chain: &BallStickChain<StorybookTreeChain>) -> Vec<StickNode> {
	let mut branch_index = 0usize;
	chain
		.segments_with_hysteresis()
		.filter_map(|(segment, parent, _)| {
			if !keep_branch_on_low(is_stalk(parent), &mut branch_index) {
				return None;
			}
			stick_node_for_segment(&segment, parent)
		})
		.collect()
}
