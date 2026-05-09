//! Chico ball component.
//! Simply wraps the noisy ball and provides a [FromScalarNoise] impl.

use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

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

impl RenderItem for ChicoBall {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		_cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		vec![commands.spawn((self.clone(), transform)).id()]
	}
}
