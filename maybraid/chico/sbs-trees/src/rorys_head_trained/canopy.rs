//! Small layered-ball canopy at every graph joint (stalk and projection limbs).

use bevy::prelude::Vec3;
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	BallStickNode, StorybookTreeChain,
};
use chico_vegetation_components::{FoliageNode, Placement};

/// Medium foliage: denser azimuth × height outer samples.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

/// Slightly undersized vs node radius so limb gaps stay covered without dominating sticks.
const JOINT_CANOPY_BALL_SCALE: f32 = 0.88;

fn foliage_node_for_joint(node: &BallStickNode, leaf_radius_world: f32) -> FoliageNode {
	let scale = (leaf_radius_world / node.radius.max(1e-4)) * JOINT_CANOPY_BALL_SCALE;
	let world_radius = node.radius * scale;
	FoliageNode::layered_ball(Placement::foliage_uniform(node.position, world_radius))
}

pub(crate) fn foliage_nodes_high(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	chain
		.nodes()
		.map(|node| foliage_node_for_joint(node, leaf_radius_world))
		.collect()
}

#[derive(Clone, Copy)]
struct JointCandidate {
	position: Vec3,
	radius: f32,
}

/// Outermost joints per azimuth × height cell → layered balls.
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<StorybookTreeChain>,
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates: Vec<JointCandidate> = chain
		.nodes()
		.map(|node| JointCandidate { position: node.position, radius: node.radius })
		.collect();
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(&candidates, |c| c.position, bands);
	sampled
		.into_iter()
		.map(|s| {
			let scale = (leaf_radius_world / s.item.radius.max(1e-4)) * JOINT_CANOPY_BALL_SCALE;
			let world_radius = s.item.radius * scale;
			FoliageNode::layered_ball(Placement::foliage_uniform(s.item.position, world_radius))
		})
		.collect()
}
