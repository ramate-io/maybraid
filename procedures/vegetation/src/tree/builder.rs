use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use comproc::{
	complex::chain::ball_stick::{
		builder::{BallStick, BallStickBuilder},
		render::{mesh_handle_stack::MeshHandleStackSpawner, BallStickRenderItem},
	},
	noise::config::NoiseConfig,
};
use noise::{NoiseFn, Seedable};
use render_item::mesh::cache::mesh::disk::DiskMeshCache;
use render_item::{
	mesh::{
		cache::handle::map::HandleMap, handle::MeshHandle, IdentifiedMesh, MeshBuilder,
		MeshDispatch,
	},
	NormalizeChunk, RenderItem,
};
use std::fmt::Debug;

pub trait MeshFromTreeNum: MeshBuilder + NormalizeChunk + IdentifiedMesh {
	fn from_tree_num(tree_num: f32) -> Self;
}

#[derive(Component, Debug, Clone)]
pub struct Tree<
	BallMesh: MeshFromTreeNum,
	StickMesh: MeshFromTreeNum,
	LeafMesh: MeshFromTreeNum,
	StickMaterial: Material,
	LeafMaterial: Material,
> {
	anchor: Vec3,
	height: f32,
	stick_material: MeshMaterial3d<StickMaterial>,
	// currentlly we don't need to use this at the spawning stage
	pub leaf_material: MeshMaterial3d<LeafMaterial>,
	trunk_meshes: Vec<MeshHandle<StickMesh>>,
	branch_ball_sticks: Vec<BallStick>,
	branch_spawner: MeshHandleStackSpawner<BallMesh, StickMesh, StickMaterial>,
	leaf_spawner: MeshHandleStackSpawner<LeafMesh, LeafMesh, LeafMaterial>,
}

impl<
		BallMesh: MeshFromTreeNum,
		StickMesh: MeshFromTreeNum,
		LeafMesh: MeshFromTreeNum,
		StickMaterial: Material,
		LeafMaterial: Material,
	> Tree<BallMesh, StickMesh, LeafMesh, StickMaterial, LeafMaterial>
where
	(CascadeChunk, MeshDispatch<MeshHandle<StickMesh>>, Transform, MeshMaterial3d<StickMaterial>):
		Bundle,
{
	pub fn centroid_anchor(&self) -> Vec3 {
		let pivot_offset = Vec3::new(0.5, 0.0, 0.5);
		self.anchor - pivot_offset * Vec3::new(1.0, 1.0, 1.0)
	}

	pub fn spawn_trunk(&self, commands: &mut Commands, cascade_chunk: &CascadeChunk) {
		// Build tree segment dispatch
		if let Some(mesh_handle) = self.trunk_meshes.get(0) {
			commands.spawn((
				CascadeChunk::unit_center_chunk().with_res_2(3),
				MeshDispatch::new(mesh_handle.clone()),
				Transform::from_translation(self.centroid_anchor() + Vec3::new(0.0, 0.0, 0.0))
					.with_scale(Vec3::new(1.0, self.height / 2.0, 1.0)),
				MeshMaterial3d(self.stick_material.0.clone()),
			));

			commands.spawn((
				cascade_chunk.clone(),
				MeshDispatch::new(mesh_handle.clone()),
				Transform::from_translation(self.centroid_anchor()).with_scale(Vec3::new(
					0.9,
					self.height,
					0.9,
				)),
				MeshMaterial3d(self.stick_material.0.clone()),
			));
		};
	}
}

impl<
		BallMesh: MeshFromTreeNum,
		StickMesh: MeshFromTreeNum,
		LeafMesh: MeshFromTreeNum,
		StickMaterial: Material,
		LeafMaterial: Material,
	> RenderItem for Tree<BallMesh, StickMesh, LeafMesh, StickMaterial, LeafMaterial>
where
	(CascadeChunk, MeshDispatch<MeshHandle<BallMesh>>, Transform, MeshMaterial3d<StickMaterial>):
		Bundle,
	(CascadeChunk, MeshDispatch<MeshHandle<StickMesh>>, Transform, MeshMaterial3d<StickMaterial>):
		Bundle,
	(CascadeChunk, MeshDispatch<MeshHandle<LeafMesh>>, Transform, MeshMaterial3d<LeafMaterial>):
		Bundle,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let mut entities = Vec::new();
		if cascade_chunk.res_2 < 5 {
			// just spawn one big leaf ball for the canopy
			let mesh = LeafMesh::from_tree_num(0.0);
			let mesh_handle = MeshHandle::new(mesh);

			let ball_transform = Transform::from_translation(
				transform.translation + Vec3::new(0.0, self.height * 0.75, 0.0),
			)
			.with_scale(Vec3::new(self.height * 0.6, self.height * 0.5, self.height * 0.6));

			commands.spawn((
				cascade_chunk.clone(),
				MeshDispatch::new(mesh_handle.clone()),
				ball_transform,
				MeshMaterial3d(self.leaf_material.0.clone()),
			));
		} else {
			for branch in &self.branch_ball_sticks {
				let branch_render_item =
					BallStickRenderItem::new(branch.clone(), self.branch_spawner.clone());
				entities.extend(branch_render_item.spawn_render_items(
					commands,
					cascade_chunk,
					transform,
				));

				let (ballstick, _spawner) = branch_render_item.into_parts();
				let leaf_render_item =
					BallStickRenderItem::new(ballstick.clone(), self.leaf_spawner.clone());
				entities.extend(leaf_render_item.spawn_render_items(
					commands,
					cascade_chunk,
					transform,
				));
			}
		}

		self.spawn_trunk(commands, cascade_chunk);

		entities
	}
}

pub struct TreeBuilder<
	BallMesh: MeshFromTreeNum,
	StickMesh: MeshFromTreeNum,
	LeafMesh: MeshFromTreeNum,
	N: NoiseFn<f64, 4> + Seedable + Debug + Clone,
	M: NoiseFn<f64, 3> + Seedable + Debug + Clone,
	StickMaterial: Material,
	LeafMaterial: Material,
> {
	pub anchor: Vec3,
	pub height: f32,
	pub branch_count: usize,
	pub leaf_ball_scale: Vec3,
	pub noise_config_3d: NoiseConfig<3, M>,
	pub noise_config_4d: NoiseConfig<4, N>,
	pub ball_variety: u32,
	pub ball_cache: HandleMap<BallMesh>,
	pub ball_mesh_cache: Option<DiskMeshCache<BallMesh>>,
	pub stick_variety: u32,
	pub stick_cache: HandleMap<StickMesh>,
	pub stick_mesh_cache: Option<DiskMeshCache<StickMesh>>,
	pub leaf_variety: u32,
	pub leaf_cache: HandleMap<LeafMesh>,
	pub stick_material: MeshMaterial3d<StickMaterial>,
	pub leaf_material: MeshMaterial3d<LeafMaterial>,
}

impl<
		BallMesh: MeshFromTreeNum,
		StickMesh: MeshFromTreeNum,
		LeafMesh: MeshFromTreeNum,
		StickMaterial: Material,
		LeafMaterial: Material,
		N: NoiseFn<f64, 4> + Seedable + Debug + Clone,
		M: NoiseFn<f64, 3> + Seedable + Debug + Clone,
	> TreeBuilder<BallMesh, StickMesh, LeafMesh, N, M, StickMaterial, LeafMaterial>
{
	pub fn get_branch_height(&self, last_position: Vec3) -> f32 {
		let noise_value = self.noise_config_3d.vec3_on_unit(last_position) as f32;
		(noise_value * self.height).min(self.height)
	}

	pub fn branch_builder(&self, anchor: Vec3, initial_ray: Vec3) -> BallStickBuilder<N, M> {
		BallStickBuilder::common_tree_builder()
			.with_anchor(anchor)
			.with_initial_ray(initial_ray)
			.with_bias_ray(initial_ray + Vec3::new(0.0, 0.01, 0.0))
			.with_bias_amount(0.2)
			.with_angle_tolerance(2.0)
			.with_splitting_coefficient(0.6)
			.with_min_segment_length(0.6)
			.with_max_segment_length(1.4)
			.with_min_radius(0.1)
			.with_max_radius(0.2)
			.with_depth(4)
			.with_noise_config_3d(self.noise_config_3d.clone())
			.with_noise_config_4d(self.noise_config_4d.clone())
	}

	pub fn compute_radial_branches(&self) -> Vec<BallStick> {
		let mut branches = Vec::new();
		let pre_height = self.get_branch_height(self.anchor);
		let mut last_position = self.anchor + Vec3::new(0.0, pre_height, 0.0);

		for i in 0..self.branch_count {
			let height = self.get_branch_height(last_position);
			let angle = i as f32 * 2.0 * std::f32::consts::PI / self.branch_count as f32;
			let initial_ray =
				Vec3::new(angle.cos(), angle.sin() + angle.cos(), angle.sin()).normalize();

			let branch_builder = self.branch_builder(last_position, initial_ray);
			let branch = branch_builder.build();
			branches.push(branch);

			let biased_height = height.max(last_position.y - height);
			last_position = self.anchor + Vec3::new(0.0, biased_height, 0.0);
			last_position.y = last_position.y.min(self.height);
		}

		branches
	}

	pub fn tree_num(&self) -> f32 {
		self.noise_config_3d.vec3_on_unit(self.anchor) as f32
	}

	pub fn build(self) -> Tree<BallMesh, StickMesh, LeafMesh, StickMaterial, LeafMaterial> {
		let branch_ball_sticks = self.compute_radial_branches();
		let tree_num = self.tree_num();

		let stick_meshes: Vec<MeshHandle<StickMesh>> = (0..self.stick_variety)
			.map(|i| {
				MeshHandle::new(StickMesh::from_tree_num(tree_num + i as f32))
					.with_handle_cache(self.stick_cache.clone())
					.with_mesh_cache(self.stick_mesh_cache.clone())
			})
			.collect();

		let ball_meshes: Vec<MeshHandle<BallMesh>> = (0..self.ball_variety)
			.map(|i| {
				MeshHandle::new(BallMesh::from_tree_num(tree_num + i as f32))
					.with_handle_cache(self.ball_cache.clone())
					.with_mesh_cache(self.ball_mesh_cache.clone())
			})
			.collect();

		let leaf_meshes: Vec<MeshHandle<LeafMesh>> = (0..self.leaf_variety)
			.map(|i| {
				MeshHandle::new(LeafMesh::from_tree_num(tree_num + i as f32))
					.with_handle_cache(self.leaf_cache.clone())
			})
			.collect();

		let branch_spawner =
			MeshHandleStackSpawner::new(self.stick_material.clone(), self.stick_material.clone())
				.with_stick_mesh_handle_stack(stick_meshes.clone())
				.with_ball_mesh_handle_stack(ball_meshes.clone());

		let leaf_spawner =
			MeshHandleStackSpawner::new(self.leaf_material.clone(), self.leaf_material.clone())
				.with_ball_mesh_handle_stack(leaf_meshes.clone())
				.with_ball_scale(self.leaf_ball_scale);

		Tree {
			anchor: self.anchor,
			height: self.height,
			stick_material: self.stick_material,
			leaf_material: self.leaf_material,
			trunk_meshes: stick_meshes.clone(),
			branch_ball_sticks,
			branch_spawner,
			leaf_spawner,
		}
	}
}
