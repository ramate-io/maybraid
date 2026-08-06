//! Cheap-ball joint canopy plus optional stalk-tip apex ([#232](https://github.com/ramate-io/maybraid/issues/232)).
//!
//! Medium uses ~30% fewer band cells. Medium and Low share a thin layered proxy that
//! spans the full canopy height (top-to-bottom) with reduced XZ extent.

use bevy::prelude::Vec3;
use chico_sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use chico_sbs_geometry::{
	liams_stalk_tip_from_chain, sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands,
	BallStickChain, LiamsConiferChain,
};
use chico_vegetation_components::{FoliageNode, Placement};

/// Needle-cluster world radius as a fraction of stalk height (RFC `0.018 * H`; playground default wider).
pub const NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.048;

/// Kept for Params/CLI defaults (historical plane-splay sizing; samples are cheap balls now).
pub const NORTHERN_SPLAY_CORE_RADIUS: f32 = 0.75;
pub const NORTHERN_SPLAY_LEAF_DISC_RADIUS: f32 = 0.95;

/// High foliage: densest azimuth × height outer samples.
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage: ~30% fewer cells than the prior 24×8 grid.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(17, 8);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

/// Horizontal (XZ) scale of the thin full-height canopy proxy.
const THIN_PROXY_XZ_SCALE: f32 = 0.40;

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
}

fn world_ball_radius(c: &FoliageCandidate, splay_radius_world: f32) -> f32 {
	let scale = splay_radius_world / c.radius.max(1e-4);
	c.radius * scale
}

fn foliage_node_from_candidate(c: &FoliageCandidate, splay_radius_world: f32) -> FoliageNode {
	FoliageNode::cheap_ball(Placement::foliage_uniform(
		c.position,
		world_ball_radius(c, splay_radius_world),
	))
}

fn collect_candidates(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_spawn_fraction: f32,
) -> Vec<FoliageCandidate> {
	chain
		.nodes_with_hysteresis_enumerated()
		.filter_map(|(idx, node, _)| {
			if !mix_seed_below_fraction(idx, node.position, splay_spawn_fraction) {
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
	splay_radius_world: f32,
) -> Vec<FoliageNode> {
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(candidates, |c| c.position, bands);
	sampled
		.into_iter()
		.map(|s| foliage_node_from_candidate(s.item, splay_radius_world))
		.collect()
}

fn maybe_apex_ball(
	chain: &BallStickChain<LiamsConiferChain>,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Option<FoliageNode> {
	let tip = liams_stalk_tip_from_chain(chain);
	if !mix_seed_below_fraction(0, tip.position, apex_spawn_fraction) {
		return None;
	}
	Some(FoliageNode::cheap_ball(Placement::foliage_uniform(
		tip.position,
		apex_radius_world,
	)))
}

/// Thin layered proxy spanning full canopy height (reduced XZ footprint).
fn thin_full_height_proxy_ball(
	candidates: &[FoliageCandidate],
	splay_radius_world: f32,
) -> Option<FoliageNode> {
	if candidates.is_empty() {
		return None;
	}
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	for c in candidates {
		let r = world_ball_radius(c, splay_radius_world);
		let p = c.position;
		min = min.min(p - Vec3::splat(r));
		max = max.max(p + Vec3::splat(r));
	}
	let center = (min + max) * 0.5;
	let mut half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	half_extents.x *= THIN_PROXY_XZ_SCALE;
	half_extents.z *= THIN_PROXY_XZ_SCALE;
	Some(FoliageNode::layered_ball(
		Placement::new(center, 0.0).with_scale(half_extents),
	))
}

fn with_proxy_and_apex(
	candidates: &[FoliageCandidate],
	bands: AzimuthHeightBands,
	splay_radius_world: f32,
	chain: &BallStickChain<LiamsConiferChain>,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let mut nodes = banded_from_candidates(candidates, bands, splay_radius_world);
	if let Some(proxy) = thin_full_height_proxy_ball(candidates, splay_radius_world) {
		nodes.push(proxy);
	}
	if let Some(apex) = maybe_apex_ball(chain, apex_spawn_fraction, apex_radius_world) {
		nodes.push(apex);
	}
	nodes
}

/// Banded joint cheap-ball foliage; apex when spawn fraction passes (High: no mass proxy).
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<LiamsConiferChain>,
	bands: AzimuthHeightBands,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, splay_spawn_fraction);
	let mut nodes = banded_from_candidates(&candidates, bands, splay_radius_world);
	if let Some(apex) = maybe_apex_ball(chain, apex_spawn_fraction, apex_radius_world) {
		nodes.push(apex);
	}
	nodes
}

/// Medium samples (~30% fewer cells) plus thin full-height proxy and optional apex.
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, splay_spawn_fraction);
	with_proxy_and_apex(
		&candidates,
		MEDIUM_FOLIAGE_BANDS,
		splay_radius_world,
		chain,
		apex_spawn_fraction,
		apex_radius_world,
	)
}

/// Coarse samples plus the same thin full-height proxy and optional apex.
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, splay_spawn_fraction);
	with_proxy_and_apex(
		&candidates,
		LOW_FOLIAGE_BANDS,
		splay_radius_world,
		chain,
		apex_spawn_fraction,
		apex_radius_world,
	)
}
