//! Selective torch canopy: cheap balls on upper/outer BranchOut nodes.
//!
//! High / Medium outer-sample the candidate set. Low uses a coarser sample plus one
//! canopy-extent proxy ball.

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
/// Low foliage: coarser sample (proxy ball supplies the bulk silhouette).
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

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

fn world_ball_radius(c: &FoliageCandidate, leaf_radius_world: f32) -> f32 {
	let scale = leaf_radius_world / c.radius.max(1e-4);
	c.radius * scale
}

fn foliage_node_from_candidate(c: &FoliageCandidate, leaf_radius_world: f32) -> FoliageNode {
	FoliageNode::cheap_ball(Placement::foliage_uniform(
		c.position,
		world_ball_radius(c, leaf_radius_world),
	))
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

/// One layered ball matching the axis-aligned canopy extents (unit ball → AABB).
fn canopy_extent_proxy_ball(
	candidates: &[FoliageCandidate],
	leaf_radius_world: f32,
) -> Option<FoliageNode> {
	if candidates.is_empty() {
		return None;
	}
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	for c in candidates {
		let r = world_ball_radius(c, leaf_radius_world);
		let p = c.position;
		min = min.min(p - Vec3::splat(r));
		max = max.max(p + Vec3::splat(r));
	}
	let center = (min + max) * 0.5;
	let half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	Some(FoliageNode::layered_ball(
		Placement::new(center, 0.0).with_scale(half_extents),
	))
}

/// Coarse outer samples plus one canopy-extent proxy ball.
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(&candidates, |c| c.position, LOW_FOLIAGE_BANDS);
	let mut nodes: Vec<FoliageNode> = sampled
		.into_iter()
		.map(|s| foliage_node_from_candidate(s.item, leaf_radius_world))
		.collect();
	if let Some(proxy) = canopy_extent_proxy_ball(&candidates, leaf_radius_world) {
		nodes.push(proxy);
	}
	nodes
}
