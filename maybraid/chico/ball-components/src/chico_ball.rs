//! Chico ball component.
//! Simply wraps the noisy ball and provides a [FromScalarNoise] impl.

pub mod render_item_plugin;

use bevy::prelude::*;
use chico_sdf::{Ball, NoisySurface};
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{mesh::handle::Cached, CascadeChunk, RenderItem};

/// Simple canopy ball marker item for first-pass tree assembly.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChicoBall {
	pub seed_scalar: f32,
	pub frequency: f32,
	pub amplitude: f32,
	pub octaves: u32,
}

impl Default for ChicoBall {
	fn default() -> Self {
		Self { seed_scalar: 0.0, frequency: 1.0, amplitude: 1.0, octaves: 1 }
	}
}

impl FromScalarNoise for ChicoBall {
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self { seed_scalar, frequency, amplitude, octaves }
	}
}

impl ChicoBall {
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

impl RenderItem for ChicoBall {
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

		vec![commands
			.spawn((
				Cached::new(self.noisy_ball()),
				cascade_chunk.clone(),
				transform.with_translation(translation),
				MeshMaterial3d::<StandardMaterial>::default(),
			))
			.id()]
	}
}
