//! **Chico stick**: noisy tapered cylinder along +Y for ball-stick **segment** meshes.
//!
//! # Role in Sope's Banyan ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252))
//!
//! Sticks are the mesh primitive for each graph edge between parent and child nodes once a `BallStickChain` (and render helpers in `chico-sbs-geometry`) supply segment transforms. Bark-facing materials in the RFC (dark / wet / high-contrast fantasy bark) attach at the tree or playground layer; this crate stays the **reusable stick component** with a `FromScalarNoise` implementation for procedural variation.

use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

/// First-pass stick marker.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct ChicoStick {
	pub seed_scalar: f32,
	pub frequency: f32,
	pub amplitude: f32,
	pub octaves: u32,
}

impl Default for ChicoStick {
	fn default() -> Self {
		Self { seed_scalar: 0.0, frequency: 1.0, amplitude: 1.0, octaves: 1 }
	}
}

impl FromScalarNoise for ChicoStick {
	fn from_scalar(seed_scalar: f32, frequency: f32, amplitude: f32, octaves: u32) -> Self {
		Self { seed_scalar, frequency, amplitude, octaves }
	}
}

impl RenderItem for ChicoStick {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		_cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		vec![commands.spawn((self.clone(), transform)).id()]
	}
}
