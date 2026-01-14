use crate::tree::builder::{Tree, TreeBuilder};
use crate::tree::meshes::canopy::ball::NoisyBall;
use crate::tree::meshes::trunk::segment::SimpleTrunkSegment;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use comproc::noise::config::NoiseConfig;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::RenderItem;

use noise::Perlin;

#[derive(Component, Clone)]
pub struct GroveBuilder<T: Material, L: Material> {
	noise_config_3d: NoiseConfig<3, Perlin>,
	noise_config_4d: NoiseConfig<4, Perlin>,
	threshold: f32,
	anchor: Vec3,
	step_size: f32,
	count: usize,
	trunk_material: MeshMaterial3d<T>,
	leaf_material: MeshMaterial3d<L>,
	tree_cache: HandleMap<SimpleTrunkSegment>,
	leaf_cache: HandleMap<NoisyBall>,
	min_height: f32,
	max_height: f32,
}

impl<T: Material, L: Material> GroveBuilder<T, L> {
	pub fn new(trunk_material: MeshMaterial3d<T>, leaf_material: MeshMaterial3d<L>) -> Self {
		Self {
			noise_config_3d: NoiseConfig::default(),
			noise_config_4d: NoiseConfig::default(),
			threshold: 0.5,
			anchor: Vec3::ZERO,
			step_size: 4.0,
			count: 16,
			trunk_material,
			leaf_material,
			tree_cache: HandleMap::new(),
			leaf_cache: HandleMap::new(),
			min_height: 2.0,
			max_height: 6.0,
		}
	}

	pub fn with_tree_cache(mut self, tree_cache: HandleMap<SimpleTrunkSegment>) -> Self {
		self.tree_cache = tree_cache;
		self
	}

	pub fn with_leaf_cache(mut self, leaf_cache: HandleMap<NoisyBall>) -> Self {
		self.leaf_cache = leaf_cache;
		self
	}

	pub fn meets_threshold(&self, position: Vec3) -> bool {
		let noise = self.noise_config_3d.vec3_on_unit(position);
		noise as f32 > self.threshold
	}

	pub fn inner_noise(&self, position: Vec3) -> f32 {
		self.noise_config_3d.vec3_amp(position) as f32 * self.step_size / 2.0
	}

	pub fn get_height(&self, position: Vec3) -> f32 {
		let noise = self.noise_config_3d.vec3_on_unit(position);
		noise as f32 * (self.max_height - self.min_height) + self.min_height
	}

	pub fn build(&self) -> Grove<T, L> {
		let mut trees = Vec::new();
		for i in 0..self.count {
			for j in 0..self.count {
				let pre_position = self.anchor
					+ Vec3::new(i as f32 * self.step_size, 0.0, j as f32 * self.step_size);

				let position = pre_position
					+ Vec3::new(
						self.inner_noise(pre_position),
						0.0,
						self.inner_noise(pre_position),
					);

				if self.meets_threshold(position) {
					let tree_builder = TreeBuilder {
						anchor: position,
						height: self.get_height(position),
						branch_count: 4,
						leaf_ball_scale: Vec3::new(1.0, 1.0, 1.0),
						noise_config_3d: self.noise_config_3d.clone(),
						noise_config_4d: self.noise_config_4d.clone(),
						ball_variety: 0,
						ball_cache: self.leaf_cache.clone(),
						stick_variety: 1,
						stick_cache: self.tree_cache.clone(),
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
		let mut entities = Vec::new();
		for (position, tree) in &self.trees {
			let transform = transform.with_translation(*position);
			entities.extend(tree.spawn_render_items(commands, cascade_chunk, transform));
		}
		entities
	}
}
