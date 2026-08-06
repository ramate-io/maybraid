//! Terminal canopy: plane-splay on outer / terminal BranchOut joints, with torch-like LOD banding.
//!
//! Low adds a mid-canopy layered mass proxy.

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

/// Medium sticks: ~20% more cells than shared torch medium (10×4 → 12×4).
pub(crate) const MEDIUM_STICK_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);

/// Crown-height window for the Low mid-canopy proxy.
const PROXY_CROWN_Y_START: f32 = 0.15;
const PROXY_CROWN_Y_END: f32 = 0.80;
const PROXY_HEIGHT_SCALE: f32 = 0.80;

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

/// Mid-crown layered AABB proxy (fills Low mass).
fn mid_canopy_proxy_ball(
	candidates: &[FoliageCandidate],
	leaf_radius_world: f32,
) -> Option<FoliageNode> {
	if candidates.is_empty() {
		return None;
	}
	let mut y_min = f32::INFINITY;
	let mut y_max = f32::NEG_INFINITY;
	for c in candidates {
		y_min = y_min.min(c.position.y);
		y_max = y_max.max(c.position.y);
	}
	let span = (y_max - y_min).max(1e-4);
	let y_lo = y_min + span * PROXY_CROWN_Y_START;
	let y_hi = y_min + span * PROXY_CROWN_Y_END;

	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut mid_any = false;
	for c in candidates {
		let y = c.position.y;
		if y < y_lo || y > y_hi {
			continue;
		}
		let r = world_splay_radius(c, leaf_radius_world);
		let p = c.position;
		min = min.min(p - Vec3::splat(r));
		max = max.max(p + Vec3::splat(r));
		mid_any = true;
	}
	if !mid_any {
		return None;
	}
	let center = (min + max) * 0.5;
	let mut half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	half_extents.y *= PROXY_HEIGHT_SCALE;
	Some(FoliageNode::layered_ball(
		Placement::new(center, 0.0).with_scale(half_extents),
	))
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

/// Coarse outer samples plus a mid-canopy layered proxy.
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	let mut nodes = banded_from_candidates(&candidates, LOW_FOLIAGE_BANDS, leaf_radius_world);
	if let Some(proxy) = mid_canopy_proxy_ball(&candidates, leaf_radius_world) {
		nodes.push(proxy);
	}
	nodes
}
