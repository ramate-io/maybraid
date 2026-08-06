//! Selective torch canopy: layered balls + plane-splay on upper/outer BranchOut nodes.

use bevy::prelude::Vec3;
use chico_sbs_geometry::render::mix_seed::node_mix_seed;
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	StorybookTreeChain, StorybookTreePhase,
};
use chico_vegetation_components::{FoliageGeometry, FoliageNode, Placement};

/// Medium foliage: denser azimuth × height outer samples.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

const PLANE_SPLAY_CORE_RADIUS: f32 = 0.75;

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

fn prefers_plane_splay(ring_u: f32, node_idx: usize, position: Vec3) -> bool {
	if ring_u > 0.35 {
		return true;
	}
	let seed = node_mix_seed(node_idx, position);
	seed % 10 < 4
}

#[derive(Clone, Copy)]
struct FoliageCandidate {
	node_idx: usize,
	position: Vec3,
	radius: f32,
	ring_u: f32,
}

fn foliage_node_from_candidate(c: &FoliageCandidate, leaf_radius_world: f32) -> FoliageNode {
	let scale = leaf_radius_world / c.radius.max(1e-4);
	let world_radius = c.radius * scale;
	let placement = Placement::foliage_uniform(c.position, world_radius);
	if prefers_plane_splay(c.ring_u, c.node_idx, c.position) {
		let seed = node_mix_seed(c.node_idx, c.position);
		let subdiv = seed % 2;
		let disc_r = 0.18 + 0.12 * ((seed % 17) as f32 / 16.0);
		FoliageNode::plane_splay(
			FoliageGeometry::plane_splay(subdiv, PLANE_SPLAY_CORE_RADIUS, disc_r),
			placement,
		)
	} else {
		FoliageNode::layered_ball(placement)
	}
}

fn collect_high_candidates(
	chain: &BallStickChain<StorybookTreeChain>,
) -> Vec<FoliageCandidate> {
	chain
		.nodes_with_hysteresis_enumerated()
		.filter_map(|(idx, node, h)| {
			if !should_allocate_foliage(idx, h, chain) {
				return None;
			}
			Some(FoliageCandidate {
				node_idx: idx,
				position: node.position,
				radius: node.radius,
				ring_u: h.ring_u,
			})
		})
		.collect()
}

pub(crate) fn foliage_nodes_high(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	collect_high_candidates(chain)
		.iter()
		.map(|c| foliage_node_from_candidate(c, leaf_radius_world))
		.collect()
}

/// Outermost high-policy foliage candidates per azimuth × height cell.
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<StorybookTreeChain>,
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_high_candidates(chain);
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(&candidates, |c| c.position, bands);
	sampled
		.into_iter()
		.map(|s| foliage_node_from_candidate(s.item, leaf_radius_world))
		.collect()
}
