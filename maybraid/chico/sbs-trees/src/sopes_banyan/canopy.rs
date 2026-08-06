//! Terminal canopy: layered-ball foliage nodes (with structural LOD filters).

use chico_sbs_geometry::{AzimuthHeightBands, BallStickChain, BallStickNode, SopesBanyanChain};
use chico_vegetation_components::{FoliageNode, Placement};

/// Medium foliage: denser azimuth × height outer samples (preserves vase pinch).
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

pub(crate) fn foliage_node_for_terminal(
	node: &BallStickNode,
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

/// Outermost chain nodes per azimuth × height cell → layered balls (crown floor filtered).
pub(crate) fn banded_outer_canopy_balls(
	chain: &BallStickChain<SopesBanyanChain>,
	bands: AzimuthHeightBands,
	min_height: f32,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	chain
		.sample_radius_azimuth(bands)
		.into_iter()
		.filter_map(|sample| {
			foliage_node_for_terminal(sample.item, min_height, leaf_radius_world)
		})
		.collect()
}
