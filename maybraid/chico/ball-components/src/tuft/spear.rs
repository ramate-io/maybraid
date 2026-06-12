//! **Spear tuft** — thin flat grass-like spears ([#217](https://github.com/ramate-io/maybraid/issues/217)).
//!
//! 2D ribbon blades with belly→tip width profile; mesh construction in [`construction`].

pub mod construction;

use std::marker::PhantomData;

use bevy::prelude::*;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use super::directions::CapDirections;
use super::profile::BellyTipProfile;
use super::spawn::MergedTuft;

pub use construction::{SpearCluster, SpearElement};

/// [`StandardMaterial`] spear tuft (common default).
pub type SpearTuftStd = SpearTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / noise-driven shape parameters for [`SpearTuft`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SpearTuftShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 12))]
	pub spear_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.9))]
	pub spear_length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.008))]
	pub base_half_width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.022))]
	pub belly_half_width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.25))]
	pub max_tilt_radians: f32,
	/// Along-strand segment count (`1` = one straight section base→tip; higher = more kinks).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 2))]
	pub bend_segments: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.08))]
	pub noise_amplitude: f32,
	/// Sway cycles **per bend segment**; near `1.0` each segment kinks independently, lower
	/// keeps neighbouring segments correlated (smoother bow).
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub noise_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for SpearTuftShape {
	fn default() -> Self {
		Self {
			spear_count: 12,
			spear_length: 0.9,
			base_half_width: 0.008,
			belly_half_width: 0.022,
			max_tilt_radians: 0.25,
			bend_segments: 2,
			noise_amplitude: 0.08,
			noise_frequency: 1.0,
			seed: 0,
		}
	}
}

/// Upward flat grass spears (2D ribbons).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SpearTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: SpearTuftShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for SpearTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			shape: SpearTuftShape::default(),
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for SpearTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(noise: NoiseParams) -> Self {
		Self {
			shape: SpearTuftShape {
				seed: noise.seed,
				noise_frequency: noise.frequency,
				noise_amplitude: noise.amplitude,
				..SpearTuftShape::default()
			},
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> SpearTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: SpearTuftShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn spear_directions(&self) -> Vec<Vec3> {
		CapDirections::upward(self.shape.spear_count, self.shape.seed, self.shape.max_tilt_radians)
	}

	fn spear_length_at(&self, index: u32, min: f32, max: f32, scale: f32) -> f32 {
		(self.shape.spear_length * CapDirections::length_scale(index, self.shape.seed, min, max)
			* scale)
			.max(1e-4)
	}

	pub fn build_mesh(&self, world_uniform_scale: f32) -> Mesh {
		let scale = world_uniform_scale.max(1e-8);
		let profile = BellyTipProfile {
			base_half_width: (self.shape.base_half_width * scale).max(1e-6),
			belly_half_width: (self.shape.belly_half_width * scale)
				.max(self.shape.base_half_width * scale),
		};
		let noise_amplitude = self.shape.noise_amplitude * scale;

		let elements: Vec<SpearElement> = self
			.spear_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| SpearElement {
				direction,
				length: self.spear_length_at(i as u32, 0.78, 1.05, scale),
				profile,
				seed: self.shape.seed.wrapping_add(i as i32),
			})
			.collect();

		SpearCluster::new(
			elements,
			self.shape.bend_segments,
			self.shape.noise_frequency,
			noise_amplitude,
		)
		.into_mesh()
	}
}

impl<M: Material, S> MergedTuft for SpearTuft<M, S>
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

impl<M: Material, S> RenderItem for SpearTuft<M, S>
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
