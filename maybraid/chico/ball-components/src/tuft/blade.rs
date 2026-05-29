//! **Blade tuft** — thin, flat, grass-like blades (sketch; [#217](https://github.com/ramate-io/maybraid/issues/217)).

use std::marker::PhantomData;

use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

use super::directions::CapDirections;
use super::prism::{PrismaticCluster, PrismaticElement};
use super::spawn::MergedTuft;

const HEIGHT_SEGMENTS: u32 = 5;
const SIDE_COUNT: u32 = 2;

/// [`StandardMaterial`] blade tuft (common default).
pub type BladeTuftStd = BladeTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / noise-driven shape parameters for [`BladeTuft`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct BladeTuftShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 12))]
	pub blade_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.15))]
	pub blade_length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.025))]
	pub blade_width: f32,
	/// Max polar angle from +Y (radians); keep small for columnar grass clumps.
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.22))]
	pub max_tilt_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.10))]
	pub noise_amplitude: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 5.0))]
	pub noise_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for BladeTuftShape {
	fn default() -> Self {
		Self {
			blade_count: 12,
			blade_length: 1.15,
			blade_width: 0.025,
			max_tilt_radians: 0.22,
			noise_amplitude: 0.10,
			noise_frequency: 5.0,
			seed: 0,
		}
	}
}

/// Thin flat blades radiating from a shared anchor (sketch implementation via ribbon prisms).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BladeTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: BladeTuftShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for BladeTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			shape: BladeTuftShape::default(),
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for BladeTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, _octaves: u32) -> Self {
		Self {
			shape: BladeTuftShape {
				seed: seed_scalar as i32,
				noise_frequency: frequency,
				noise_amplitude: amplitude,
				..BladeTuftShape::default()
			},
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> BladeTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: BladeTuftShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn blade_directions(&self) -> Vec<Vec3> {
		CapDirections::upward(self.shape.blade_count, self.shape.seed, self.shape.max_tilt_radians)
	}

	fn blade_length_at(&self, index: u32, min: f32, max: f32, scale: f32) -> f32 {
		(self.shape.blade_length * CapDirections::length_scale(index, self.shape.seed, min, max)
			* scale)
			.max(1e-4)
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let half_width = (self.shape.blade_width * scale * 0.5).max(1e-6);
		let tip_width = half_width * 0.15;
		let noise_amplitude = self.shape.noise_amplitude * scale;

		let elements: Vec<PrismaticElement> = self
			.blade_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| PrismaticElement {
				direction,
				length: self.blade_length_at(i as u32, 0.78, 1.0, scale),
				base_radius: half_width,
				tip_radius: tip_width,
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

impl<M: Material, S> MergedTuft for BladeTuft<M, S>
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

impl<M: Material, S> RenderItem for BladeTuft<M, S>
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
