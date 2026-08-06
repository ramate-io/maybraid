//! Terminal canopy: plane-splay on outer / terminal BranchOut joints, with torch-like LOD banding.

use bevy::prelude::Vec3;
use chico_sbs_geometry::chain::storybook_tree::is_graph_terminal;
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	StorybookTreeChain, StorybookTreePhase,
};
use chico_vegetation_components::{FoliageGeometry, FoliageNode, Placement};

/// High foliage: densest azimuth × height outer samples.
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

fn should_allocate_plane_splay(
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
	let is_terminal = is_graph_terminal(chain, node_idx);
	let outer = hysteresis.distance_from_anchor
		> hysteresis.outer_foliage_distance_fraction * hysteresis.projection_length;
	is_terminal || outer
}

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
}

fn world_splay_radius(c: &FoliageCandidate, leaf_radius_world: f32) -> f32 {
	let scale = leaf_radius_world / c.radius.max(1e-4);
	c.radius * scale
}

fn foliage_node_from_candidate(c: &FoliageCandidate, leaf_radius_world: f32) -> FoliageNode {
	FoliageNode::plane_splay(
		FoliageGeometry::default_plane_splay(),
		Placement::foliage_uniform(c.position, world_splay_radius(c, leaf_radius_world)),
	)
}

fn collect_candidates(chain: &BallStickChain<StorybookTreeChain>) -> Vec<FoliageCandidate> {
	chain
		.nodes_with_hysteresis_enumerated()
		.filter_map(|(idx, node, h)| {
			if !should_allocate_plane_splay(idx, h, chain) {
				return None;
			}
			Some(FoliageCandidate {
				position: node.position,
				radius: node.radius,
			})
		})
		.collect()
}

fn banded_from_candidates(
	candidates: &[FoliageCandidate],
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(candidates, |c| c.position, bands);
	sampled
		.into_iter()
		.map(|s| foliage_node_from_candidate(s.item, leaf_radius_world))
		.collect()
}

/// Outermost foliage candidates per azimuth × height cell.
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<StorybookTreeChain>,
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	banded_from_candidates(&candidates, bands, leaf_radius_world)
}

/// Medium outer samples (no mass proxy).
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(chain, MEDIUM_FOLIAGE_BANDS, leaf_radius_world)
}

/// Coarse outer samples (no mass proxy).
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(chain, LOW_FOLIAGE_BANDS, leaf_radius_world)
}
