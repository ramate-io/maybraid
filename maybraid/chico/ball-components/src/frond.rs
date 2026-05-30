//! **Frond crown** — arching leaflet chains for palms and ferns ([#218](https://github.com/ramate-io/maybraid/issues/218)).
//!
//! Mesh construction lives in [`construction`]; per-strand geometry in [`config`].

pub mod config;
pub mod construction;
pub mod crown;
pub mod leaflet;
pub mod render_item_plugin;
pub mod spawn;
pub mod spine;

use std::marker::PhantomData;

use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

use config::FrondConfig;
use crown::{crown_directions, length_scale};
use spawn::MergedFrond;

pub use config::FrondConfig as FrondGeometry;
pub use construction::{FrondCluster, FrondElement};
pub use render_item_plugin::FrondRenderItemPlugin;

/// [`StandardMaterial`] frond crown (common default).
pub type FrondCrownStd = FrondCrown<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / shape parameters for a merged frond crown cluster.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct FrondCrownShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 9))]
	pub frond_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.4))]
	pub length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.18))]
	pub width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.55))]
	pub droop: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.35))]
	pub twist: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 12))]
	pub leaflet_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 8))]
	pub spine_segments: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.62))]
	pub downward_tilt_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.48))]
	pub outward_spread_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for FrondCrownShape {
	fn default() -> Self {
		Self {
			frond_count: 9,
			length: 1.4,
			width: 0.18,
			droop: 0.55,
			twist: 0.35,
			leaflet_count: 12,
			spine_segments: 8,
			downward_tilt_radians: 0.62,
			outward_spread_radians: 0.48,
			seed: 0,
		}
	}
}

impl FrondCrownShape {
	pub fn frond_config(&self, scale: f32) -> FrondConfig {
		FrondConfig {
			segments: self.spine_segments.max(1),
			length: (self.length * scale).max(1e-4),
			width: (self.width * scale).max(1e-6),
			droop: self.droop * scale,
			twist: self.twist,
			leaflet_count: self.leaflet_count.max(2),
		}
	}
}

/// Palm- or fern-like crown: many drooping fronds merged at one anchor.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct FrondCrown<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: FrondCrownShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for FrondCrown<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			shape: FrondCrownShape::default(),
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for FrondCrown<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(seed_scalar: f32, _frequency: f32, _amplitude: f32, _octaves: u32) -> Self {
		Self {
			shape: FrondCrownShape {
				seed: seed_scalar as i32,
				..FrondCrownShape::default()
			},
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FrondCrown<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: FrondCrownShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn frond_directions(&self) -> Vec<Vec3> {
		crown_directions(
			self.shape.frond_count,
			self.shape.seed,
			self.shape.downward_tilt_radians,
			self.shape.outward_spread_radians,
		)
	}

	fn frond_length_scale(&self, index: u32, scale: f32) -> f32 {
		length_scale(index, self.shape.seed, 0.82, 1.08) * scale
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let config = self.shape.frond_config(scale);

		let elements: Vec<FrondElement> = self
			.frond_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| {
				let mut element_config = config;
				element_config.length *= self.frond_length_scale(i as u32, 1.0);
				FrondElement {
					direction,
					config: element_config,
					seed: self.shape.seed.wrapping_add(i as i32),
				}
			})
			.collect();

		FrondCluster::new(elements).into_mesh()
	}
}

/// Single frond strand (one combined spine + leaflet mesh).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct Frond<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub config: FrondConfig,
	pub direction: Vec3,
	pub seed: i32,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Frond<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let mut config = self.config;
		config.length = (config.length * scale).max(1e-4);
		config.width = (config.width * scale).max(1e-6);
		config.droop *= scale;

		FrondCluster::new(vec![FrondElement {
			direction: self.direction,
			config,
			seed: self.seed,
		}])
		.into_mesh()
	}
}

impl<M: Material, S> MergedFrond for FrondCrown<M, S>
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

impl<M: Material, S> MergedFrond for Frond<M, S>
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

impl<M: Material, S> RenderItem for FrondCrown<M, S>
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

impl<M: Material, S> RenderItem for Frond<M, S>
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

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::mesh::VertexAttributeValues;

	#[test]
	fn crown_defaults_droop_outward() -> Result<()> {
		let crown = FrondCrown::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::default();
		for d in crown.frond_directions() {
			assert!(d.y < 0.0, "palm fronds should droop downward: {d:?}");
		}
		Ok(())
	}

	#[test]
	fn crown_mesh_is_non_empty() -> Result<()> {
		let crown = FrondCrown::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::default();
		let mesh = crown.build_mesh(1.0);
		let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!(!pos.is_empty());
		Ok(())
	}
}
