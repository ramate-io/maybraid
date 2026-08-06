//! Terminal canopy: layered-ball foliage nodes (with structural LOD filters).

use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickNode,
	SopesBanyanChain,
};
use chico_vegetation_components::{FoliageNode, Placement};

/// Medium foliage: denser azimuth × height outer samples (preserves vase pinch).
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

pub(crate) fn foliage_node_for_terminal(
	_node_idx: usize,
	node: &BallStickNode,
	_hysteresis: &SopesBanyanChain,
	min_height: f32,
	leaf_radius_world: f32,
) -> Option<FoliageNode> {
	if node.position.y < min_height {
		return None;
	}
	let scale = leaf_radius_world / node.radius.max(1e-4);
	let radius = node.radius * scale;
	let placement = Placement::foliage_uniform(node.position, radius);
	Some(FoliageNode::layered_ball(placement))
}

/// Outermost High foliage per azimuth × height cell, collapsed to layered balls.
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
	.map(|sample| FoliageNode::layered_ball(sample.item.placement))
	.collect()
}
