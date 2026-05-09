//! Plane splay ball component.
//! Implements https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/05-plane-splay
//!

use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

/// First-pass plane-splay canopy marker.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct PlaneSplay {
	pub seed_scalar: f32,
	pub frequency: f32,
	pub amplitude: f32,
	pub octaves: u32,
}

impl Default for PlaneSplay {
	fn default() -> Self {
		Self { seed_scalar: 0.0, frequency: 1.0, amplitude: 1.0, octaves: 1 }
	}
}

impl FromScalarNoise for PlaneSplay {
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self { seed_scalar, frequency, amplitude, octaves }
	}
}

impl RenderItem for PlaneSplay {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		_cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		vec![commands.spawn((self.clone(), transform)).id()]
	}
}
