//! Terminal canopy: mix NoisyBall and PlaneSplay foliage nodes (with structural LOD filters).

use chico_sbs_geometry::render::mix_seed::node_mix_seed;
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickNode,
	SopesBanyanChain, SopesBanyanPhase,
};
use chico_vegetation_components::{FoliageGeometry, FoliageNode, Placement};

/// Medium foliage: denser azimuth × height outer samples (preserves vase pinch).
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

/// Prefer plane splay in the rising crown; stay mostly on noisy balls along descenders.
fn canopy_prefers_plane_splay(
	node_idx: usize,
	node: &BallStickNode,
	hysteresis: &SopesBanyanChain,
) -> bool {
	let descender_leaning = matches!(
		&hysteresis.phase,
		SopesBanyanPhase::StartDescender(_) | SopesBanyanPhase::EndDescender(_)
	);
	let seed = node_mix_seed(node_idx, node.position);
	if descender_leaning {
		seed % 13 < 2
	} else {
		seed % 10 < 5
	}
}

pub(crate) fn foliage_node_for_terminal(
	node_idx: usize,
	node: &BallStickNode,
	hysteresis: &SopesBanyanChain,
	min_height: f32,
	leaf_radius_world: f32,
) -> Option<FoliageNode> {
	if node.position.y < min_height {
		return None;
	}
	let scale = leaf_radius_world / node.radius.max(1e-4);
	let radius = node.radius * scale;
	let placement = Placement::foliage_uniform(node.position, radius);

	if canopy_prefers_plane_splay(node_idx, node, hysteresis) {
		let seed = node_mix_seed(node_idx, node.position);
		let geom = FoliageGeometry::plane_splay(
			seed % 2,
			0.8,
			0.18 + 0.12 * ((seed % 17) as f32 / 16.0),
		);
		Some(FoliageNode::plane_splay(geom, placement))
	} else {
		Some(FoliageNode::noisy_ball(placement))
	}
}

/// Outermost High foliage per azimuth × height cell, collapsed to noisy balls.
pub(crate) fn banded_outer_canopy_balls(
	high_foliage: &[FoliageNode],
	bands: AzimuthHeightBands,
) -> Vec<FoliageNode> {
	sample_max_horizontal_radius_by_azimuth_height(
		high_foliage,
		|node| node.placement.translation,
		bands,
	)
	.into_iter()
	.map(|sample| FoliageNode::noisy_ball(sample.item.placement))
	.collect()
}
