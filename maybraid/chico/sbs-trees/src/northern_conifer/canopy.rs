//! Plane-splay at ball-stick joints plus optional stalk-tip apex ball ([#232](https://github.com/ramate-io/maybraid/issues/232)).

use bevy::prelude::Vec3;
use chico_sbs_geometry::render::mix_seed::{mix_seed_below_fraction, node_mix_seed};
use chico_sbs_geometry::{
	liams_stalk_tip_from_chain, sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands,
	BallStickChain, LiamsConiferChain,
};
use chico_vegetation_components::{FoliageGeometry, FoliageNode, Placement};

/// Needle-cluster world radius as a fraction of stalk height (RFC `0.018 * H`; playground default wider).
pub const NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.048;

/// Local icosphere/plate sizing before joint scale (narrow needle clusters).
pub const NORTHERN_SPLAY_CORE_RADIUS: f32 = 0.75;
pub const NORTHERN_SPLAY_LEAF_DISC_RADIUS: f32 = 0.95;

/// High foliage: densest azimuth × height outer samples.
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

/// Map RFC `splay_count` 2..4 to icosphere subdivision (`2` = 320 faces, `3` = 1280, `4` = 5120).
fn icosphere_subdivisions_for_node(node_idx: usize, position: Vec3) -> u32 {
	let t = (node_mix_seed(node_idx, position) as f32) / (u32::MAX as f32);
	if t < 0.34 {
		2
	} else if t < 0.67 {
		3
	} else {
		4
	}
}

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
	icosphere_subdivisions: u32,
}

fn world_splay_radius(c: &FoliageCandidate, splay_radius_world: f32) -> f32 {
	let scale = splay_radius_world / c.radius.max(1e-4);
	c.radius * scale
}

fn foliage_node_from_candidate(c: &FoliageCandidate, splay_radius_world: f32) -> FoliageNode {
	FoliageNode::plane_splay(
		FoliageGeometry::plane_splay(
			c.icosphere_subdivisions,
			NORTHERN_SPLAY_CORE_RADIUS,
			NORTHERN_SPLAY_LEAF_DISC_RADIUS,
		),
		Placement::foliage_uniform(c.position, world_splay_radius(c, splay_radius_world)),
	)
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
				icosphere_subdivisions: icosphere_subdivisions_for_node(idx, node.position),
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

/// Banded joint splay foliage; apex when spawn fraction passes.
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

/// Medium outer samples plus optional apex (no mass proxy).
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(
		chain,
		MEDIUM_FOLIAGE_BANDS,
		splay_radius_world,
		splay_spawn_fraction,
		apex_spawn_fraction,
		apex_radius_world,
	)
}

/// Coarse outer samples plus optional apex (no mass proxy).
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(
		chain,
		LOW_FOLIAGE_BANDS,
		splay_radius_world,
		splay_spawn_fraction,
		apex_spawn_fraction,
		apex_radius_world,
	)
}
