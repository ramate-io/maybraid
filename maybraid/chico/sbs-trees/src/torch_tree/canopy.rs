//! Selective torch canopy: cheap balls on upper/outer BranchOut nodes.
//!
//! All structural LOD levels outer-sample the same candidate set; High is densest.

use bevy::prelude::Vec3;
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	StorybookTreeChain, StorybookTreePhase,
};
use chico_vegetation_components::{FoliageNode, Placement};

/// High foliage: densest azimuth × height outer samples (still drops near-duplicates).
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Low foliage.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);

/// RFC §3.1.7.4 ball selection: terminal, upper belt, or far along limb.
fn should_allocate_foliage(
	node_idx: usize,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	let is_terminal = chain.children.get(node_idx).is_some_and(|c| c.is_empty());
	let upper = hysteresis.ring_u > 0.55;
	let outer = hysteresis.distance_from_anchor > 0.70 * hysteresis.projection_length;
	is_terminal || upper || outer
}

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
}

fn foliage_node_from_candidate(c: &FoliageCandidate, leaf_radius_world: f32) -> FoliageNode {
	let scale = leaf_radius_world / c.radius.max(1e-4);
	let world_radius = c.radius * scale;
	FoliageNode::cheap_ball(Placement::foliage_uniform(c.position, world_radius))
}

fn collect_candidates(chain: &BallStickChain<StorybookTreeChain>) -> Vec<FoliageCandidate> {
	chain
		.nodes_with_hysteresis_enumerated()
		.filter_map(|(idx, node, h)| {
			if !should_allocate_foliage(idx, h, chain) {
				return None;
			}
			Some(FoliageCandidate { position: node.position, radius: node.radius })
		})
		.collect()
}

/// Outermost foliage candidates per azimuth × height cell.
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<StorybookTreeChain>,
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(&candidates, |c| c.position, bands);
	sampled
		.into_iter()
		.map(|s| foliage_node_from_candidate(s.item, leaf_radius_world))
		.collect()
}
