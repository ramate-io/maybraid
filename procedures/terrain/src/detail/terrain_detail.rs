use crate::Terrain;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use comproc::noise::config::NoiseConfig;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::cache::mesh::disk::DiskMeshCache;
use render_item::mesh::MeshDispatch;
use render_item::mesh::{handle::MeshHandle, IdentifiedMesh, MeshBuilder};
use render_item::RenderItem;

use noise::Perlin;

pub trait MeshFromTerrainDetailNum: MeshBuilder + IdentifiedMesh {
	fn from_terrain_detail_num(terrain_detail_num: f32) -> Self;
}

#[derive(Component, Clone)]
pub struct TerrainDetail<D: MeshFromTerrainDetailNum, M: Material, E: Terrain + Clone> {
	noise_config_3d: NoiseConfig<3, Perlin>,
	noise_config_4d: NoiseConfig<4, Perlin>,
	threshold: f32,
	anchor: Vec3,
	step_size: Vec2,
	detail_material: MeshMaterial3d<M>,
	detail_handle_cache: HandleMap<D>,
	detail_mesh_cache: Option<DiskMeshCache<D>>,
	min_radii: Vec3,
	max_radii: Vec3,
	terrain: E,
}

impl<D: MeshFromTerrainDetailNum, M: Material, E: Terrain + Clone> TerrainDetail<D, M, E> {
	pub fn new(detail_material: MeshMaterial3d<M>, terrain: E) -> Self {
		Self {
			noise_config_3d: NoiseConfig::default(),
			noise_config_4d: NoiseConfig::default(),
			threshold: 0.5,
			anchor: Vec3::ZERO,
			step_size: Vec2::new(6.0, 6.0),
			detail_material,
			detail_handle_cache: HandleMap::new(),
			detail_mesh_cache: None,
			min_radii: Vec3::new(0.5, 0.5, 0.5),
			max_radii: Vec3::new(6.0, 6.0, 6.0),
			terrain,
		}
	}

	pub fn with_step_size(mut self, step_size: Vec2) -> Self {
		self.step_size = step_size;
		self
	}

	pub fn with_detail_handle_cache(mut self, detail_handle_cache: HandleMap<D>) -> Self {
		self.detail_handle_cache = detail_handle_cache;
		self
	}

	pub fn with_detail_mesh_cache(mut self, detail_mesh_cache: Option<DiskMeshCache<D>>) -> Self {
		self.detail_mesh_cache = detail_mesh_cache;
		self
	}

	pub fn with_anchor(mut self, anchor: Vec3) -> Self {
		self.anchor = anchor;
		self
	}

	pub fn meets_threshold(&self, position: Vec3) -> bool {
		let noise = self.noise_config_3d.vec3_on_unit(position);
		noise as f32 > self.threshold
	}

	pub fn inner_noise(&self, position: Vec3) -> Vec2 {
		self.noise_config_3d.vec3_amp(position) as f32 * self.step_size * 2.0
	}

	pub fn get_terrain_height(&self, x: f32, z: f32) -> f32 {
		self.terrain.composed_height_at(x, z)
	}

	pub fn get_terrain_laplacian(&self, x: f32, z: f32, step: f32) -> f32 {
		self.terrain.laplacian_at(x, z, step)
	}

	pub fn get_terrain_slope_mag(&self, x: f32, z: f32, step: f32) -> f32 {
		self.terrain.slope_mag(x, z, step)
	}

	pub fn get_terrain_slope_angle_deg(&self, x: f32, z: f32, step: f32) -> f32 {
		self.terrain.slope_angle_deg(x, z, step)
	}

	pub fn get_scale(&self, position: Vec3) -> Vec3 {
		let noise = self.noise_config_3d.vec3_on_unit(position);
		Vec3::new(
			noise as f32 * (self.max_radii.x - self.min_radii.x) + self.min_radii.x,
			noise as f32 * (self.max_radii.y - self.min_radii.y) + self.min_radii.y,
			noise as f32 * (self.max_radii.z - self.min_radii.z) + self.min_radii.z,
		)
	}

	pub fn get_x_z_noisy_position(&self, xz: Vec2) -> Vec2 {
		let noise = self.noise_config_3d.vec3_on_unit(Vec3::new(xz.x, xz.x + xz.y, xz.y));
		Vec2::new(xz.x + noise as f32 * self.step_size.x, xz.y + noise as f32 * self.step_size.y)
	}

	pub fn get_noise_num(&self, xz: Vec3) -> f32 {
		self.noise_config_3d.vec3_on_unit(xz) as f32
	}
}

impl<D: MeshFromTerrainDetailNum, M: Material, E: Terrain + Clone> RenderItem
	for TerrainDetail<D, M, E>
where
	(CascadeChunk, MeshDispatch<MeshHandle<D>>, Transform, MeshMaterial3d<M>): Bundle,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		if cascade_chunk.res_2 < 1 {
			return vec![];
		}

		// iterate through every step within the bounds of the
		let mut x = cascade_chunk.origin.x;
		while x <= cascade_chunk.origin.x + cascade_chunk.size {
			let mut z = cascade_chunk.origin.z;
			while z <= cascade_chunk.origin.z + cascade_chunk.size {
				let elevation = self.get_terrain_height(x, z);
				let noisy_position = self.get_x_z_noisy_position(Vec2::new(x, z));
				let position = Vec3::new(noisy_position.x, elevation, noisy_position.y);

				if self.meets_threshold(position) {
					let scale = self.get_scale(position);
					let transform = Transform::from_translation(position).with_scale(scale);

					let mesh_builder = D::from_terrain_detail_num(self.get_noise_num(position));
					let mesh_handle = MeshHandle::new(mesh_builder)
						.with_handle_cache(self.detail_handle_cache.clone());

					commands.spawn((
						cascade_chunk.clone(),
						MeshDispatch::new(mesh_handle),
						transform,
						self.detail_material.clone(),
					));
				}
				z += self.step_size.y;
			}
			x += self.step_size.x;
		}
		vec![]
	}
}
