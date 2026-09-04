//! Terrain origin cell with development-pad elevation ops applied.

use avian3d::prelude::RigidBody;
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham_terrain::shaders::DurhamTerrainShader;
use durham_terrain_models::terrain::ElevationModulation;
use durham_terrain_models::{
	cascade_chunk_for_cell, ComposedTerrain, Terrain, TerrainMeshBuilder, TerrainSdf,
	TerrainTrimeshCollider,
};
use lod::gen::{
	cull_non_adjacent_bands, Id, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus,
};
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
		}
	}

	pub fn mesh_builder(&self) -> TerrainMeshBuilder {
		CpuShotBuilder::new(Arc::clone(&self.sdf)).with_wall_faces(self.wall_faces)
	}

	pub fn scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = Transform::from_translation(chunk.origin);
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<DurhamTerrainShader>({material.clone()})
			template(move |_ctx| Ok(RigidBody::Static))
			TerrainTrimeshCollider
		}
	}

	fn center(&self) -> Vec3 {
		(Vec3::from(self.cell.min) + Vec3::from(self.cell.max)) * 0.5
	}

	fn edge(&self) -> f32 {
		let size = Vec3::from(self.cell.max) - Vec3::from(self.cell.min);
		size.x.max(size.z).max(1e-3)
	}

	fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		let delta = viewer.translation - self.center();
		let factor = Vec2::new(delta.x, delta.z).length() / self.edge();
		if factor <= 3.0 {
			LodSceneLevel::High
		} else if factor <= 7.0 {
			LodSceneLevel::Medium
		} else if factor <= 14.0 {
			LodSceneLevel::Low
		} else {
			LodSceneLevel::UltraLow
		}
	}

	fn res_2_for_level(&self, level: LodSceneLevel) -> u8 {
		match level {
			LodSceneLevel::High => self.res_2.max(3),
			LodSceneLevel::Medium => self.res_2.saturating_sub(2).max(2),
			LodSceneLevel::Low => self.res_2.saturating_sub(3).max(1),
			LodSceneLevel::UltraLow => 1,
			LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => self.res_2,
		}
	}

	fn level_scene(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2_for_level(level));
		let transform = Transform::from_translation(chunk.origin - self.center());
		let builder = self.mesh_builder();
		let material = self.material.clone();
		if level == LodSceneLevel::High {
			Box::new(bsn! {
				template_value(transform)
				template_value(chunk)
				template(move |_ctx| Ok(Cached::new(builder.clone())))
				MeshMaterial3d::<DurhamTerrainShader>({material.clone()})
				template(move |_ctx| Ok(RigidBody::Static))
				TerrainTrimeshCollider
			})
		} else {
			Box::new(bsn! {
				template_value(transform)
				template_value(chunk)
				template(move |_ctx| Ok(Cached::new(builder.clone())))
				MeshMaterial3d::<DurhamTerrainShader>({material.clone()})
			})
		}
	}
}

impl LodScene for TerrainWithPads {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let previous = self.level_for(lod_ref.previous_transform);
		let current = self.level_for(lod_ref.current_transform);
		if previous == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		cull_non_adjacent_bands(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.level_scene(level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		let center = self.center();
		Aabb3d::from_min_max(Vec3::from(self.cell.min) - center, Vec3::from(self.cell.max) - center)
	}
}

/// Marks a spawned padded-terrain scene root.
#[derive(Component, Debug, Clone, Copy)]
pub struct PresentedPaddedTerrainScene(pub Id);
