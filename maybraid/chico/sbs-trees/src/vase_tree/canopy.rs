//! Vase canopy: outer plane-splay / inner cheap-ball on upper joints, plus stalk-tip apex ball.
//!
//! High / Medium / Low band joint candidates together; apex is always emitted.

use bevy::prelude::Vec3;
use chico_sbs_geometry::chain::storybook_tree::is_graph_terminal;
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, stalk_tip_from_chain, AzimuthHeightBands,
	BallStickChain, StorybookTreeChain, StorybookTreePhase,
};
use chico_vegetation_components::{FoliageGeometry, FoliageNode, Placement};

/// High foliage: densest azimuth × height outer samples.
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

fn qualifies_for_foliage(
	node_idx: usize,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
	upper_foliage_ring_u: f32,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	is_graph_terminal(chain, node_idx)
		|| hysteresis.ring_u > upper_foliage_ring_u
		|| hysteresis.distance_from_anchor
			> hysteresis.outer_foliage_distance_fraction * hysteresis.projection_length.max(1e-6)
}

fn is_outer_foliage(
	node_idx: usize,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
) -> bool {
	is_graph_terminal(chain, node_idx)
		|| hysteresis.distance_from_anchor
			> hysteresis.outer_foliage_distance_fraction * hysteresis.projection_length.max(1e-6)
}

#[derive(Clone, Copy)]
enum FoliageKit {
	OuterSplay,
	InnerBall,
}

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
	kit: FoliageKit,
}

fn world_leaf_radius(c: &FoliageCandidate, leaf_radius_world: f32) -> f32 {
	let scale = leaf_radius_world / c.radius.max(1e-4);
	c.radius * scale
}

fn foliage_node_from_candidate(c: &FoliageCandidate, leaf_radius_world: f32) -> FoliageNode {
	let placement = Placement::foliage_uniform(c.position, world_leaf_radius(c, leaf_radius_world));
	match c.kit {
		FoliageKit::OuterSplay => {
			FoliageNode::plane_splay(FoliageGeometry::default_plane_splay(), placement)
		}
		FoliageKit::InnerBall => FoliageNode::cheap_ball(placement),
	}
}

fn collect_candidates(
	chain: &BallStickChain<StorybookTreeChain>,
	upper_foliage_ring_u: f32,
) -> Vec<FoliageCandidate> {
	chain
		.nodes_with_hysteresis_enumerated()
		.filter_map(|(idx, node, h)| {
			if !qualifies_for_foliage(idx, h, chain, upper_foliage_ring_u) {
				return None;
			}
			let kit = if is_outer_foliage(idx, h, chain) {
				FoliageKit::OuterSplay
			} else {
				FoliageKit::InnerBall
			};
			Some(FoliageCandidate {
				position: node.position,
				radius: node.radius,
				kit,
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

fn apex_ball(chain: &BallStickChain<StorybookTreeChain>, apex_radius_world: f32) -> FoliageNode {
	let tip = stalk_tip_from_chain(chain);
	FoliageNode::cheap_ball(Placement::foliage_uniform(tip.position, apex_radius_world))
}

/// Banded joint foliage plus apex ball (always).
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<StorybookTreeChain>,
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
	upper_foliage_ring_u: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, upper_foliage_ring_u);
	let mut nodes = banded_from_candidates(&candidates, bands, leaf_radius_world);
	nodes.push(apex_ball(chain, apex_radius_world));
	nodes
}

/// Medium outer samples plus apex (no mass proxy).
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
	upper_foliage_ring_u: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(
		chain,
		MEDIUM_FOLIAGE_BANDS,
		leaf_radius_world,
		upper_foliage_ring_u,
		apex_radius_world,
	)
}

/// Coarse outer samples plus apex (no mass proxy).
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
	upper_foliage_ring_u: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(
		chain,
		LOW_FOLIAGE_BANDS,
		leaf_radius_world,
		upper_foliage_ring_u,
		apex_radius_world,
	)
}
