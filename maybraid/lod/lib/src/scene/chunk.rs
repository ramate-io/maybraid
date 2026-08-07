//! Incremental LOD scene composition ([`SceneChunk`]).
//!
//! A chunk tree is a **scheduling** representation only: it does not change
//! scene semantics. Fulfillment flattens to weighted primitives and drains them
//! under a per-frame weight budget (see [`crate::scene::chunk_fulfill`]).
//!
//! # Future work: coalescing and compaction
//!
//! Many tiny weights can dominate scheduling overhead. Follow-ups worth adding:
//!
//! - **Coalescing** — merge adjacent cheap primitives (or subtrees under a weight
//!   floor) into one spawn unit before enqueue.
//! - **Compaction** — rewrite a deep `SubChunks` tree into a flatter primitive
//!   list (or balanced fan-out) so the drain loop stays cheap.
//!
//! The initial implementation flattens eagerly at job start; lazy materialization
//! of subtrees is also intentionally out of scope for now.

use bevy::scene::prelude::Scene;
use std::collections::VecDeque;

/// Default weight for [`SceneChunk::primitive`].
pub const DEFAULT_CHUNK_WEIGHT: u32 = 1;

/// Scheduling tree for incremental LOD fulfillment.
pub enum SceneChunk {
	/// Nested scheduling groups (flattened before drain).
	SubChunks(Vec<SceneChunk>),
	/// One spawn unit with a relative cost heuristic.
	Primitive { weight: u32, scene: Box<dyn Scene> },
}

impl SceneChunk {
	/// Single primitive with [`DEFAULT_CHUNK_WEIGHT`].
	pub fn primitive(scene: impl Scene + 'static) -> Self {
		Self::weighted(DEFAULT_CHUNK_WEIGHT, scene)
	}

	/// Single primitive with an explicit weight (relative heuristic).
	pub fn weighted(weight: u32, scene: impl Scene + 'static) -> Self {
		Self::Primitive { weight: weight.max(1), scene: Box::new(scene) }
	}

	/// Group child chunks.
	pub fn chunks(chunks: impl IntoIterator<Item = SceneChunk>) -> Self {
		Self::SubChunks(chunks.into_iter().collect())
	}

	/// Flatten into a FIFO of `(weight, scene)` primitives.
	pub fn into_primitives(self) -> VecDeque<(u32, Box<dyn Scene>)> {
		let mut out = VecDeque::new();
		self.collect_primitives(&mut out);
		out
	}

	fn collect_primitives(self, out: &mut VecDeque<(u32, Box<dyn Scene>)>) {
		match self {
			Self::SubChunks(children) => {
				for child in children {
					child.collect_primitives(out);
				}
			}
			Self::Primitive { weight, scene } => {
				out.push_back((weight.max(1), scene));
			}
		}
	}

	/// Total weight of all primitives (after flatten).
	pub fn total_weight(&self) -> u32 {
		match self {
			Self::SubChunks(children) => children.iter().map(Self::total_weight).sum(),
			Self::Primitive { weight, .. } => (*weight).max(1),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::scene::{ResolveContext, ResolvedScene};

	fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

	#[test]
	fn flatten_preserves_order_and_weights() {
		let chunk = SceneChunk::chunks([
			SceneChunk::weighted(2, bevy::scene::SceneFunction(empty_scene)),
			SceneChunk::chunks([SceneChunk::primitive(bevy::scene::SceneFunction(empty_scene))]),
			SceneChunk::weighted(5, bevy::scene::SceneFunction(empty_scene)),
		]);
		assert_eq!(chunk.total_weight(), 8);
		let prims = chunk.into_primitives();
		assert_eq!(prims.len(), 3);
		assert_eq!(prims[0].0, 2);
		assert_eq!(prims[1].0, DEFAULT_CHUNK_WEIGHT);
		assert_eq!(prims[2].0, 5);
	}
}
