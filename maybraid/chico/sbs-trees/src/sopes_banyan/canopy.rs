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

pub(crate) fn horizontal_radius(position: Vec3) -> f32 {
	Vec3::new(position.x, 0.0, position.z).length()
}

/// Medium: keep canopy in the outer half of the footprint (silhouette), as noisy balls.
pub(crate) fn outer_half_canopy_balls(
	high_foliage: &[FoliageNode],
	tree_radius: f32,
) -> Vec<FoliageNode> {
	let threshold = tree_radius.max(1e-4) * 0.5;
	high_foliage
		.iter()
		.filter(|node| horizontal_radius(node.placement.translation) >= threshold)
		.map(|node| FoliageNode::noisy_ball(node.placement))
		.collect()
}

fn azimuth_sector(position: Vec3) -> Option<usize> {
	let r = horizontal_radius(position);
	if r < 1e-4 {
		return None;
	}
	let azimuth = position.z.atan2(position.x);
	Some(
		(((azimuth + std::f32::consts::PI) / (std::f32::consts::FRAC_PI_2)).floor() as usize)
			.min(3),
	)
}

/// Slight oversize so four balls overlap and read as one crown mass.
const LOW_CANOPY_FILL: f32 = 1.2;

/// Low: four noisy balls — one per azimuth quadrant, scaled to fill that quadrant's canopy AABB.
pub(crate) fn four_quadrant_canopy_balls(high_foliage: &[FoliageNode]) -> Vec<FoliageNode> {
	let mut sector_aabb: [Option<(Vec3, Vec3)>; 4] = [None, None, None, None];
	for node in high_foliage {
		let c = node.placement.translation;
		let Some(sector) = azimuth_sector(c) else {
			continue;
		};
		let e = node
			.placement
			.scale
			.x
			.abs()
			.max(node.placement.scale.y.abs())
			.max(node.placement.scale.z.abs());
		let min = c - Vec3::splat(e);
		let max = c + Vec3::splat(e);
		sector_aabb[sector] = Some(match sector_aabb[sector] {
			None => (min, max),
			Some((prev_min, prev_max)) => (prev_min.min(min), prev_max.max(max)),
		});
	}

	sector_aabb
		.into_iter()
		.flatten()
		.map(|(min, max)| {
			let center = (min + max) * 0.5;
			let half = (max - min) * 0.5;
			let radius = half.x.max(half.y).max(half.z).max(1e-3) * LOW_CANOPY_FILL;
			FoliageNode::noisy_ball(Placement::foliage_uniform(center, radius))
		})
		.collect()
}
