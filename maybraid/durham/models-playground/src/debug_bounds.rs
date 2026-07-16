//! Debug visualization of Terrain cascade-chunk bounds (SDF sample volume).

use bevy::prelude::*;
use durham_terrain_models::{cascade_chunk_for_cell, Terrain};

/// Draw a wire AABB for each generated Terrain cell's cascade chunk — the same
/// origin/extent used for SDF meshing / sampling.
pub fn draw_chunk_boundary_boxes(mut gizmos: Gizmos, terrains: Query<&Terrain>) {
	let color = Color::srgb(1.0, 0.2, 0.25);
	for terrain in &terrains {
		let chunk = cascade_chunk_for_cell(terrain.cell, terrain.res_2);
		let extent = chunk.extent_vec();
		let aabb = bevy::math::bounding::Aabb3d::from_min_max(
			chunk.origin,
			chunk.origin + extent,
		);
		gizmos.aabb_3d(aabb, Transform::IDENTITY, color);
	}
}
