//! **Buddha's-hand tuft** — clustered widening diamond fingers ([#217](https://github.com/ramate-io/maybraid/issues/217)).
//!
//! Mesh construction is in [`construction`].

pub mod construction;

use std::marker::PhantomData;

use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

use super::directions::CapDirections;
use super::profile::BellyTipProfile;
use super::spawn::MergedTuft;

const HEIGHT_SEGMENTS: u32 = 5;

pub use construction::{BuddhaHandCluster, BuddhaHandElement};

/// [`StandardMaterial`] Buddha's-hand tuft (common default).
pub type BuddhaHandTuftStd =
	BuddhaHandTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// CLI / noise-driven shape parameters for [`BuddhaHandTuft`].
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct BuddhaHandTuftShape {
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 10))]
	pub finger_count: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub finger_length: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.025))]
	pub base_half_width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.07))]
	pub belly_half_width: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.28))]
	pub max_tilt_radians: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.08))]
	pub noise_amplitude: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 4.0))]
	pub noise_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0))]
	pub seed: i32,
}

impl Default for BuddhaHandTuftShape {
	fn default() -> Self {
		Self {
			finger_count: 10,
			finger_length: 1.0,
			base_half_width: 0.025,
			belly_half_width: 0.07,
			max_tilt_radians: 0.28,
			noise_amplitude: 0.08,
			noise_frequency: 4.0,
			seed: 0,
		}
	}
}

/// Upward clustered fingers with diamond-profile cross-section (palm-hand silhouette).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BuddhaHandTuft<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub shape: BuddhaHandTuftShape,
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for BuddhaHandTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			shape: BuddhaHandTuftShape::default(),
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for BuddhaHandTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, _octaves: u32) -> Self {
		Self {
			shape: BuddhaHandTuftShape {
				seed: seed_scalar as i32,
				noise_frequency: frequency,
				noise_amplitude: amplitude,
				..BuddhaHandTuftShape::default()
			},
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> BuddhaHandTuft<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub fn from_shape(shape: BuddhaHandTuftShape, material: S) -> Self {
		Self { shape, material, __marker: PhantomData }
	}

	fn finger_directions(&self) -> Vec<Vec3> {
		CapDirections::upward(
			self.shape.finger_count,
			self.shape.seed,
			self.shape.max_tilt_radians,
		)
	}

	fn finger_length_at(&self, index: u32, min: f32, max: f32, scale: f32) -> f32 {
		(self.shape.finger_length * CapDirections::length_scale(index, self.shape.seed, min, max)
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

		let elements: Vec<BuddhaHandElement> = self
			.finger_directions()
			.into_iter()
			.enumerate()
			.map(|(i, direction)| BuddhaHandElement {
				direction,
				length: self.finger_length_at(i as u32, 0.78, 1.05, scale),
				profile,
				seed: self.shape.seed.wrapping_add(i as i32),
			})
			.collect();

		BuddhaHandCluster::new(
			elements,
			HEIGHT_SEGMENTS,
			self.shape.noise_frequency,
			noise_amplitude,
		)
		.into_mesh()
	}
}

impl<M: Material, S> MergedTuft for BuddhaHandTuft<M, S>
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

impl<M: Material, S> RenderItem for BuddhaHandTuft<M, S>
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
