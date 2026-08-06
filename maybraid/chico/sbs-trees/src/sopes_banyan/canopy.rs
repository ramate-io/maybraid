//! Terminal canopy: layered / cheap ball foliage nodes (with structural LOD filters).

use chico_sbs_geometry::{AzimuthHeightBands, BallStickChain, BallStickNode, SopesBanyanChain};
use chico_vegetation_components::{FoliageNode, Placement};

/// High foliage: dense azimuth × height outer samples (thins near-duplicate terminals).
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Medium foliage: ~40% more cells than the prior 12×4 grid.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(17, 4);
/// Low foliage: ~40% more cells than the prior 6×2 grid (cheap-ball kit).
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanopyBallKit {
	Layered,
	Cheap,
}

pub(crate) fn foliage_node_for_terminal(
	node: &BallStickNode,
	min_height: f32,
	leaf_radius_world: f32,
	kit: CanopyBallKit,
) -> Option<FoliageNode> {
	if node.position.y < min_height {
		return None;
	}
	let scale = leaf_radius_world / node.radius.max(1e-4);
	let radius = node.radius * scale;
	let placement = Placement::foliage_uniform(node.position, radius);
	Some(match kit {
		CanopyBallKit::Layered => FoliageNode::layered_ball(placement),
		CanopyBallKit::Cheap => FoliageNode::cheap_ball(placement),
	})
}

/// Outermost chain nodes per azimuth × height cell → canopy balls (crown floor filtered).
pub(crate) fn banded_outer_canopy_balls(
	chain: &BallStickChain<SopesBanyanChain>,
	bands: AzimuthHeightBands,
	min_height: f32,
	leaf_radius_world: f32,
	kit: CanopyBallKit,
) -> Vec<FoliageNode> {
	chain
		.sample_radius_azimuth(bands)
		.into_iter()
		.filter_map(|sample| {
			foliage_node_for_terminal(sample.item, min_height, leaf_radius_world, kit)
		})
		.collect()
}
