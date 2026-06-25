//! Moderate-LOD frond crown and single-strand components.

use std::marker::PhantomData;

use bevy::prelude::*;
use render_item::{CascadeChunk, RenderItem};

use super::construction::{ModerateLodPalmFrondCluster, ModerateLodPalmFrondElement};
use super::super::config::FrondConfig;
use super::super::crown::{crown_directions, length_scale};
use super::super::spawn::MergedFrond;

/// [`StandardMaterial`] moderate-LOD frond crown.
pub type ModerateLodFrondCrownStd =
	ModerateLodFrondCrown<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / shape parameters for [`ModerateLodFrondCrown`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct ModerateLodFrondCrownShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 9))]
	pub frond_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.4))]
	pub length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.18))]
	pub width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub droop: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub arch_lift: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.35))]
	pub twist: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 28))]
	pub leaflet_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 14))]
	pub spine_segments: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.028))]
	pub shoot_half_radius: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 2.8))]
	pub leaflet_length_scale: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.62))]
	pub downward_tilt_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.48))]
	pub outward_spread_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub emission_lift_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for ModerateLodFrondCrownShape {
	fn default() -> Self {
		Self {
			frond_count: 9,
			length: 1.4,
			width: 0.18,
			droop: 0.55,
			arch_lift: 0.0,
			twist: 0.35,
			leaflet_count: 28,
			spine_segments: 14,
			shoot_half_radius: 0.028,
			leaflet_length_scale: 2.8,
			downward_tilt_radians: 0.62,
			outward_spread_radians: 0.48,
			emission_lift_radians: 0.0,
			seed: 0,
		}
	}
}

impl ModerateLodFrondCrownShape {
	pub fn frond_config(&self, scale: f32) -> FrondConfig {
		FrondConfig {
			segments: self.spine_segments.max(1),
			length: (self.length * scale).max(1e-4),
			width: (self.width * scale).max(1e-6),
			droop: self.droop * scale,
			arch_lift: self.arch_lift * scale,
			twist: self.twist,
			leaflet_count: self.leaflet_count.max(2),
		}
	}
}

/// Palm-like crown using connected rachis strips and lateral leaflet cards.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ModerateLodFrondCrown<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: ModerateLodFrondCrownShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for ModerateLodFrondCrown<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			shape: ModerateLodFrondCrownShape::default(),
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> ModerateLodFrondCrown<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: ModerateLodFrondCrownShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn frond_directions(&self) -> Vec<Vec3> {
		crown_directions(
			self.shape.frond_count,
			self.shape.seed,
			self.shape.downward_tilt_radians,
			self.shape.outward_spread_radians,
			self.shape.emission_lift_radians,
		)
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let config = self.shape.frond_config(scale);
		let shoot = (self.shape.shoot_half_radius * scale).max(1e-6);

		let elements: Vec<ModerateLodPalmFrondElement> = self
			.frond_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| {
				let mut element_config = config;
				element_config.length *=
					length_scale(i as u32, self.shape.seed, 0.82, 1.08);
				ModerateLodPalmFrondElement {
					direction,
					config: element_config,
					seed: self.shape.seed.wrapping_add(i as i32),
				}
			})
			.collect();

		ModerateLodPalmFrondCluster::new(elements, shoot, self.shape.leaflet_length_scale).into_mesh()
	}
}

/// Single low-LOD palm frond strand.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ModerateLodPalmFrond<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub config: FrondConfig,
	pub direction: Vec3,
	pub shoot_half_radius: f32,
	pub leaflet_length_scale: f32,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> ModerateLodPalmFrond<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let mut config = self.config;
		config.length = (config.length * scale).max(1e-4);
		config.width = (config.width * scale).max(1e-6);
		config.droop *= scale;
		config.arch_lift *= scale;
		let shoot = (self.shoot_half_radius * scale).max(1e-6);

		ModerateLodPalmFrondCluster::new(
			vec![ModerateLodPalmFrondElement {
				direction: self.direction,
				config,
				seed: 0,
			}],
			shoot,
			self.leaflet_length_scale,
		)
		.into_mesh()
	}
}

impl<M: Material, S> MergedFrond for ModerateLodFrondCrown<M, S>
where
	M: Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	type Mat = M;
	type MatSlot = S;

	fn material_slot(&self) -> Self::MatSlot {
		self.material.clone()
	}

	fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		Self::build_mesh(self, world_uniform_scale)
	}
}

impl<M: Material, S> MergedFrond for ModerateLodPalmFrond<M, S>
where
	M: Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	type Mat = M;
	type MatSlot = S;

	fn material_slot(&self) -> Self::MatSlot {
		self.material.clone()
	}

	fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		Self::build_mesh(self, world_uniform_scale)
	}
}

impl<M: Material, S> RenderItem for ModerateLodFrondCrown<M, S>
where
	M: Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.spawn_render_entities(commands, cascade_chunk, transform)
	}
}

impl<M: Material, S> RenderItem for ModerateLodPalmFrond<M, S>
where
	M: Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.spawn_render_entities(commands, cascade_chunk, transform)
	}
}
