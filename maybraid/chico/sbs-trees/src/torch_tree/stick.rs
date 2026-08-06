//! Storybook / torch stick → [`StickNode`] emission (with structural LOD filters).
//!
//! Torch limbs are near-vertical, so azimuth×height outer-radius banding (good for
//! wide canopies) drops most mid-band sticks. Thinning is therefore every-Nth along
//! segment order — Medium keeps every 2nd branch, Low every 4th — so Low ⊆ Medium.

use chico_sbs_geometry::{
	BallStickChain, BallStickSegment, StorybookTreeChain, StorybookTreePhase,
};
use chico_vegetation_components::{Placement, StickNode};

/// Medium: keep roughly half of branch sticks (stable every-Nth sample).
pub(crate) const MEDIUM_BRANCH_KEEP_EVERY: usize = 2;
/// Low: keep roughly a quarter of branch sticks (strict thinning of Medium).
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

fn stick_nodes_thinned(
	chain: &BallStickChain<StorybookTreeChain>,
	keep_every: usize,
) -> Vec<StickNode> {
	let keep_every = keep_every.max(1);
	let mut branch_index = 0usize;
	chain
		.segments_with_hysteresis()
		.filter_map(|(segment, parent, _)| {
			if is_stalk(parent) {
				return stick_node_for_segment(&segment, parent);
			}
			let keep = branch_index % keep_every == 0;
			branch_index += 1;
			if !keep {
				return None;
			}
			stick_node_for_segment(&segment, parent)
		})
		.collect()
}

/// Stalk always + every other branch stick.
pub(crate) fn stick_nodes_medium(chain: &BallStickChain<StorybookTreeChain>) -> Vec<StickNode> {
	stick_nodes_thinned(chain, MEDIUM_BRANCH_KEEP_EVERY)
}

/// Stalk always + every fourth branch stick (subset of Medium).
pub(crate) fn stick_nodes_low(chain: &BallStickChain<StorybookTreeChain>) -> Vec<StickNode> {
	stick_nodes_thinned(chain, LOW_BRANCH_KEEP_EVERY)
}
