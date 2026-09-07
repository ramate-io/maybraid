//! Terrain origin cell with development-pad elevation ops applied.

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::DurhamTerrainShader;
use durham_terrain_models::terrain::ElevationModulation;
use durham_terrain_models::{
	cascade_chunk_for_cell, stream_banded_level, stream_banded_scene, ComposedTerrain,
	StreamBandedLod, Terrain, TerrainCellRing, TerrainColliderMeshSource, TerrainMeshBuilder,
	TerrainSdf,
};
use lod::gen::{Id, LodScene, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use render_item::mesh::handle::Cached;
use render_item::sdf::cpu_shot::{CpuShotBuilder, WallFaces};
use std::sync::Arc;

use crate::pad::PadComplex;

/// Durham [`Terrain`] plus overlapping development pads.
#[derive(Debug, Clone, Component)]
pub struct TerrainWithPads {
	pub cell: Aabb3d,
	pub sdf: Arc<ComposedTerrain>,
	pub material: Handle<DurhamTerrainShader>,
	pub res_2: u8,
	pub wall_faces: WallFaces,
	pub pad_count: usize,
	/// Moving stream band copied from the source Durham cell.
	pub stream_ring: Option<TerrainCellRing>,
}

impl TerrainWithPads {
	pub fn compose<'a>(terrain: &Terrain, pads: impl IntoIterator<Item = &'a PadComplex>) -> Self {
		let mut sdf: TerrainSdf = terrain.sdf.terrain().clone();
		let mut pad_count = 0;
		let mut nodes = Vec::new();
		for pad in pads {
			pad_count += 1;
			nodes.extend(pad.pads.iter().cloned());
		}
		let merged = PadComplex::from_nodes(nodes);
		if !merged.is_empty() {
			sdf.add_elevation_modulation(Box::new(merged) as Box<dyn ElevationModulation>);
		}
		Self {
			cell: terrain.cell,
			sdf: Arc::new(ComposedTerrain::from_terrain(sdf)),
			material: terrain.material.clone(),
			res_2: terrain.res_2,
			// Building-skirt pads can still meet origin-cell faces on a large
			// footprint; interior skirts close the CpuShot crack.
			wall_faces: WallFaces::ALL,
			pad_count,
			stream_ring: terrain.stream_ring,
		}
	}

	pub fn mesh_builder(&self) -> TerrainMeshBuilder {
		CpuShotBuilder::new(Arc::clone(&self.sdf)).with_wall_faces(self.wall_faces)
	}

	pub fn scene(&self) -> impl Scene + 'static {
		self.mesh_scene()
	}

	/// Collider-host bake path. Visual LOD uses [`LodScene::scene_with_level`].
	pub fn collider_scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = Transform::from_translation(chunk.origin);
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<DurhamTerrainShader>({material.clone()})
			TerrainColliderMeshSource
		}
	}

	pub fn seeds_collision(&self) -> bool {
		self.stream_ring.map(|ring| ring.seeds_collision()).unwrap_or(true)
	}

	fn center(&self) -> Vec3 {
		(Vec3::from(self.cell.min) + Vec3::from(self.cell.max)) * 0.5
	}

	fn mesh_scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = Transform::from_translation(chunk.origin);
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<DurhamTerrainShader>({material.clone()})
		}
	}
}

impl StreamBandedLod for TerrainWithPads {
	fn stream_ring(&self) -> Option<TerrainCellRing> {
		self.stream_ring
	}

	fn stream_center(&self) -> Vec3 {
		self.center()
	}
}

impl LodScene for TerrainWithPads {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		stream_banded_level(self, lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let previous = stream_banded_level(self, lod_ref.previous_transform);
		let current = stream_banded_level(self, lod_ref.current_transform);
		if previous == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		stream_banded_scene(self, level, || self.mesh_scene())
	}
}

/// Marks a spawned padded-terrain scene root.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentedPaddedTerrainScene(pub Id);

#[cfg(test)]
mod tests {
	use super::*;
	use durham_terrain_models::{
		stream_banded_draws, stream_banded_level, TerrainSdf, TERRAIN_CELL_SIZE,
	};
	use lod::LodSceneLevel;

	fn far_ring() -> TerrainCellRing {
		TerrainCellRing {
			cell_size: 2.0 * TERRAIN_CELL_SIZE,
			res_2: 4,
			anchor_step: 4.0 * TERRAIN_CELL_SIZE,
			high_inner_radius: 8.0 * TERRAIN_CELL_SIZE,
			high_outer_radius: 16.0 * TERRAIN_CELL_SIZE,
			cull_margin: 8.0 * TERRAIN_CELL_SIZE,
		}
	}

	fn pad_at(center: Vec3, ring: TerrainCellRing) -> TerrainWithPads {
		let half = ring.cell_size * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, -1.0, center.z - half),
			Vec3::new(center.x + half, 1.0, center.z + half),
		);
		TerrainWithPads {
			cell: bounds,
			sdf: Arc::new(ComposedTerrain::from_terrain(TerrainSdf::new(1, 20.0))),
			material: Handle::default(),
			res_2: ring.res_2,
			wall_faces: WallFaces::NONE,
			pad_count: 0,
			stream_ring: Some(ring),
		}
	}

	#[test]
	fn far_pad_underfoot_is_empty_high() {
		let pad = pad_at(Vec3::ZERO, far_ring());
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&pad, &viewer);
		assert_eq!(level, LodSceneLevel::High);
		assert!(!stream_banded_draws(&pad, level));
	}

	#[test]
	fn far_pad_in_ring_is_drawn_medium() {
		let ring = far_ring();
		let pad = pad_at(Vec3::new(ring.high_inner_radius + 80.0, 0.0, 0.0), ring);
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&pad, &viewer);
		assert_eq!(level, LodSceneLevel::Medium);
		assert!(stream_banded_draws(&pad, level));
	}

	#[test]
	fn unbanded_pad_always_draws_high() {
		let mut pad = pad_at(Vec3::ZERO, far_ring());
		pad.stream_ring = None;
		let viewer = Transform::IDENTITY;
		assert_eq!(stream_banded_level(&pad, &viewer), LodSceneLevel::High);
		assert!(stream_banded_draws(&pad, LodSceneLevel::High));
	}
}
