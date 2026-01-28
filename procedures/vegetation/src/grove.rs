use crate::tree::builder::{Tree, TreeBuilder};
use crate::tree::meshes::canopy::ball::NoisyBall;
use crate::tree::meshes::trunk::segment::SimpleTrunkSegment;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use comproc::noise::config::NoiseConfig;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::cache::mesh::disk::DiskMeshCache;
use render_item::RenderItem;
use terrain::Terrain;

use noise::Perlin;

#[derive(Component, Clone)]
pub struct GroveBuilder<T: Material, L: Material, E: Terrain + Clone> {
	noise_config_3d: NoiseConfig<3, Perlin>,
	noise_config_4d: NoiseConfig<4, Perlin>,
	threshold: f32,
	anchor: Vec3,
	step_size: f32,
	count: usize,
	trunk_material: MeshMaterial3d<T>,
	leaf_material: MeshMaterial3d<L>,
	tree_cache: HandleMap<SimpleTrunkSegment>,
	ball_mesh_cache: Option<DiskMeshCache<NoisyBall>>,
	stick_mesh_cache: Option<DiskMeshCache<SimpleTrunkSegment>>,
	leaf_cache: HandleMap<NoisyBall>,
	min_height: f32,
	max_height: f32,
	terrain: E,
}

impl<T: Material, L: Material, E: Terrain + Clone> GroveBuilder<T, L, E> {
	pub fn new(
		trunk_material: MeshMaterial3d<T>,
		leaf_material: MeshMaterial3d<L>,
		terrain: E,
	) -> Self {
		Self {
			noise_config_3d: NoiseConfig::default(),
			noise_config_4d: NoiseConfig::default(),
			threshold: 0.5,
			anchor: Vec3::ZERO,
			step_size: 6.0,
			count: 16,
			trunk_material,
			leaf_material,
			tree_cache: HandleMap::new(),
			stick_mesh_cache: None,
			ball_mesh_cache: None,
			leaf_cache: HandleMap::new(),
			min_height: 2.0,
			max_height: 6.0,
			terrain,
		}
	}

	pub fn with_step_size(mut self, step_size: f32) -> Self {
		self.step_size = step_size;
		self
	}

	pub fn with_count(mut self, count: usize) -> Self {
		self.count = count;
		self
	}

	pub fn with_tree_cache(mut self, tree_cache: HandleMap<SimpleTrunkSegment>) -> Self {
		self.tree_cache = tree_cache;
		self
	}

	pub fn with_stick_mesh_cache(
		mut self,
		stick_mesh_cache: Option<DiskMeshCache<SimpleTrunkSegment>>,
	) -> Self {
		self.stick_mesh_cache = stick_mesh_cache;
		self
	}

	pub fn with_ball_mesh_cache(
		mut self,
		ball_mesh_cache: Option<DiskMeshCache<NoisyBall>>,
	) -> Self {
		self.ball_mesh_cache = ball_mesh_cache;
		self
	}

	pub fn with_leaf_cache(mut self, leaf_cache: HandleMap<NoisyBall>) -> Self {
		self.leaf_cache = leaf_cache;
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

	pub fn inner_noise(&self, position: Vec3) -> f32 {
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

	pub fn get_tree_height(&self, position: Vec3) -> f32 {
		let noise = self.noise_config_3d.vec3_on_unit(position);
		noise as f32 * (self.max_height - self.min_height) + self.min_height
	}

	pub fn build(&self) -> Grove<T, L> {
		let mut trees = Vec::new();
		for i in 0..self.count {
			for j in 0..self.count {
				let pre_position = self.anchor
					+ Vec3::new(
						self.anchor.x + i as f32 * self.step_size,
						self.get_terrain_height(
							self.anchor.x + i as f32 * self.step_size,
							self.anchor.z + j as f32 * self.step_size,
						),
						self.anchor.z + j as f32 * self.step_size,
					);

				// Glue the position to the terrain
				let mut position = pre_position
					+ Vec3::new(
						self.inner_noise(pre_position),
						0.0,
						self.inner_noise(pre_position),
					);
				position.y = self.get_terrain_height(position.x, position.z);

				if self.get_terrain_height(position.x, position.z) < 0.25 {
					continue;
				}

				if self.get_terrain_slope_angle_deg(position.x, position.z, self.step_size * 2.0)
					> 10.0
				{
					continue;
				}

				if self.meets_threshold(position) {
					let tree_builder = TreeBuilder {
						anchor: position,
						height: self.get_tree_height(position),
						branch_count: 4,
						leaf_ball_scale: Vec3::new(1.0, 1.0, 1.0),
						noise_config_3d: self.noise_config_3d.clone(),
						noise_config_4d: self.noise_config_4d.clone(),
						ball_variety: 0,
						ball_cache: self.leaf_cache.clone(),
						ball_mesh_cache: self.ball_mesh_cache.clone(),
						stick_variety: 1,
						stick_cache: self.tree_cache.clone(),
						stick_mesh_cache: self.stick_mesh_cache.clone(),
						leaf_variety: 1,
						leaf_cache: self.leaf_cache.clone(),
						stick_material: self.trunk_material.clone(),
						leaf_material: self.leaf_material.clone(),
					};

					let tree = tree_builder.build();

					trees.push((position, tree));
				}
			}
		}
		Grove { trees }
	}
}

#[derive(Component, Clone)]
pub struct Grove<T: Material, L: Material> {
	trees: Vec<(Vec3, Tree<NoisyBall, SimpleTrunkSegment, NoisyBall, T, L>)>,
}

impl<T: Material, L: Material> RenderItem for Grove<T, L> {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		if cascade_chunk.res_2 < 1 {
			return vec![];
		}

		let mut entities = Vec::new();
		for (position, tree) in &self.trees {
			if cascade_chunk.contains_point(*position) {
				let transform = transform.with_translation(*position);
				entities.extend(tree.spawn_render_items(commands, cascade_chunk, transform));
			}
		}
		entities
	}
}
