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

/// Low: four noisy balls — one per azimuth quadrant.
///
/// Radius is half the sector's canopy **height** (center Y span), so each ball
/// touches the top and bottom of the would-be canopy rather than ballooning from
/// the horizontal AABB. XZ sits at the mean of sector foliage centers.
pub(crate) fn four_quadrant_canopy_balls(high_foliage: &[FoliageNode]) -> Vec<FoliageNode> {
	// Per sector: sum of XZ centers, count, min/max Y of foliage centers.
	let mut sector: [Option<(Vec3, u32, f32, f32)>; 4] = [None, None, None, None];
	for node in high_foliage {
		let c = node.placement.translation;
		let Some(i) = azimuth_sector(c) else {
			continue;
		};
		sector[i] = Some(match sector[i] {
			None => (Vec3::new(c.x, 0.0, c.z), 1, c.y, c.y),
			Some((sum_xz, n, y_min, y_max)) => (
				sum_xz + Vec3::new(c.x, 0.0, c.z),
				n + 1,
				y_min.min(c.y),
				y_max.max(c.y),
			),
		});
	}

	sector
		.into_iter()
		.flatten()
		.map(|(sum_xz, n, y_min, y_max)| {
			let n = n.max(1) as f32;
			let xz = sum_xz / n;
			let y = (y_min + y_max) * 0.5;
			// Diameter = canopy height band → sphere touches top and bottom.
			let radius = ((y_max - y_min) * 0.5).max(1e-3);
			FoliageNode::noisy_ball(Placement::foliage_uniform(
				Vec3::new(xz.x, y, xz.z),
				radius,
			))
		})
		.collect()
}
