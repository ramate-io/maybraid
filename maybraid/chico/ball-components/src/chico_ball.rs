//! Chico ball component.
//!
//! Wraps the noisy ball with [`FromScalarNoise`] and an embedded material source `S` convertible via [`Into`] into [`MeshMaterial3d`].

pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sdf::{Ball, NoisySurface};
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{mesh::handle::Cached, CascadeChunk, RenderItem};

/// [`StandardMaterial`] canopy ball using explicit mesh materials (common default).
pub type ChicoBallStd = ChicoBall<StandardMaterial, MeshMaterial3d<StandardMaterial>>;

/// Simple canopy ball marker item plus embedded render material (via [`Into`]).
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChicoBall<M: Material, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	pub seed_scalar: f32,
	pub frequency: f32,
	pub amplitude: f32,
	pub octaves: u32,
	/// Converts into [`MeshMaterial3d`] at spawn (see [`Into`]).
	pub material: S,
	pub(crate) __marker: PhantomData<fn() -> M>,
}

impl<M: Material, S> Default for ChicoBall<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn default() -> Self {
		Self {
			seed_scalar: 0.0,
			frequency: 1.0,
			amplitude: 1.0,
			octaves: 1,
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> FromScalarNoise for ChicoBall<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>> + Default,
{
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self {
			seed_scalar,
			frequency,
			amplitude,
			octaves,
			material: S::default(),
			__marker: PhantomData,
		}
	}
}

impl<M: Material, S> ChicoBall<M, S>
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	/// Unit sphere with surface noise from [`FromScalarNoise`] fields.
	pub fn noisy_ball(&self) -> chico_sdf::NoisyBall {
		NoisySurface::from_params(
			Ball::unit_sphere(),
			NoiseParams::from_scalar(
				self.seed_scalar,
				self.frequency,
				self.amplitude,
				self.octaves,
			),
		)
	}
}

impl<M: Material, S> RenderItem for ChicoBall<M, S>
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
		// compute unit scale offset
		let centroid_offset =
			Vec3::new(-transform.scale.x * 0.5, -transform.scale.y * 0.5, -transform.scale.z * 0.5);
		let translation = transform.translation + centroid_offset;

		let mesh_material: MeshMaterial3d<M> = self.material.clone().into();

		vec![commands
			.spawn((
				self.clone(), // TODO: this is just a marker to help with despawning for now
				// we ill be transitioning to a better render system soon.
				Cached::new(self.noisy_ball()),
				cascade_chunk.clone(),
				transform.with_translation(translation),
				mesh_material,
			))
			.id()]
	}
}
