//! Terminal canopy: mix NoisyBall and PlaneSplay foliage nodes (with structural LOD filters).

use bevy::prelude::Vec3;
use chico_sbs_geometry::render::mix_seed::node_mix_seed;
use chico_sbs_geometry::{BallStickNode, SopesBanyanChain, SopesBanyanPhase};
use chico_vegetation_components::{FoliageGeometry, FoliageNode, Placement};

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

/// One noisy ball scaled to enclose the High canopy AABB (Low structural LOD).
pub(crate) fn canopy_extents_ball(high_foliage: &[FoliageNode]) -> Option<FoliageNode> {
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for node in high_foliage {
		let c = node.placement.translation;
		let e = node
			.placement
			.scale
			.x
			.abs()
			.max(node.placement.scale.y.abs())
			.max(node.placement.scale.z.abs());
		min = min.min(c - Vec3::splat(e));
		max = max.max(c + Vec3::splat(e));
		any = true;
	}
	if !any {
		return None;
	}
	let center = (min + max) * 0.5;
	let half = (max - min) * 0.5;
	let radius = half.x.max(half.y).max(half.z).max(1e-3);
	Some(FoliageNode::noisy_ball(Placement::foliage_uniform(center, radius)))
}
