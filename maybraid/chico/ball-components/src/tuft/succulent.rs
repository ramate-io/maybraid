//! **Succulent tuft** — short, thick, upward radiating prisms ([RFC-183 §3.1.2.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md)).
//!
//! Suited to dry conifers, succulent rosettes, and other compact canopy detail where elements read as
//! fleshy spears rather than flat grass blades.

use std::marker::PhantomData;

use bevy::prelude::*;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::directions::CapDirections;
use super::prism::{PrismaticCluster, PrismaticElement};
use super::spawn::MergedTuft;

const HEIGHT_SEGMENTS: u32 = 4;
const SIDE_COUNT: u32 = 3;

/// [`StandardMaterial`] succulent tuft (common default).
pub type SucculentTuftStd = SucculentTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / noise-driven shape parameters for [`SucculentTuft`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SucculentTuftShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 8))]
	pub element_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub element_length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.07))]
	pub base_radius: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.12))]
	pub tip_radius_fraction: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.42))]
	pub max_tilt_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.08))]
	pub noise_amplitude: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4.0))]
	pub noise_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for SucculentTuftShape {
	fn default() -> Self {
		Self {
			element_count: 8,
			element_length: 1.0,
			base_radius: 0.07,
			tip_radius_fraction: 0.12,
			max_tilt_radians: 0.42,
			noise_amplitude: 0.08,
			noise_frequency: 4.0,
			seed: 0,
		}
	}
}

/// Compact radiating tuft: thick triangular prisms with mild upward spread.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SucculentTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: SucculentTuftShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for SucculentTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			shape: SucculentTuftShape::default(),
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for SucculentTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(noise: NoiseParams) -> Self {
		Self {
			shape: SucculentTuftShape {
				seed: noise.seed,
				noise_frequency: noise.frequency,
				noise_amplitude: noise.amplitude,
				..SucculentTuftShape::default()
			},
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> SucculentTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: SucculentTuftShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn element_directions(&self) -> Vec<Vec3> {
		CapDirections::upward(
			self.shape.element_count,
			self.shape.seed,
			self.shape.max_tilt_radians,
		)
	}

	fn element_length_at(&self, index: u32, min: f32, max: f32, scale: f32) -> f32 {
		(self.shape.element_length * CapDirections::length_scale(index, self.shape.seed, min, max)
			* scale)
			.max(1e-4)
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let base_radius = (self.shape.base_radius * scale).max(1e-6);
		let tip_radius = (base_radius * self.shape.tip_radius_fraction).max(0.0);
		let noise_amplitude = self.shape.noise_amplitude * scale;

		let elements: Vec<PrismaticElement> = self
			.element_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| PrismaticElement {
				direction,
				length: self.element_length_at(i as u32, 0.72, 1.0, scale),
				base_radius,
				tip_radius,
				seed: self.shape.seed.wrapping_add(i as i32),
			})
			.collect();

		PrismaticCluster::new(
			elements,
			HEIGHT_SEGMENTS,
			SIDE_COUNT,
			self.shape.noise_frequency,
			noise_amplitude,
		)
		.into_mesh()
	}
}

impl<M: Material, S> MergedTuft for SucculentTuft<M, S>
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

impl<M: Material, S> RenderItem for SucculentTuft<M, S>
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

	#[test]
	fn defaults_are_compact_and_upward() -> Result<()> {
		let tuft =
			SucculentTuft::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::default();
		for d in tuft.element_directions() {
			assert!(d.y > 0.5);
		}
		Ok(())
	}
}
