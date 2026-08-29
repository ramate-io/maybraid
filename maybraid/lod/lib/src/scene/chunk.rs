//! Incremental LOD composition ([`LodChunk`]).
//!
//! A chunk tree is a **scheduling** representation only: it does not change
//! scene semantics. [`SemanticSceneChunk`] / [`SceneChunk`] is `Box<dyn Scene>`
//! for exclusive main-world spawn ([`crate::drain_chunk_lod_fulfill`]).
//!
//! Per-view appearance is [`crate::VisualLodScene`], not a chunk leaf.
//!
//! Fulfillment expands [`LodChunk::Lazy`] / [`LodChunk::SubChunks`] on demand.

use bevy::scene::prelude::Scene;
use std::collections::VecDeque;

/// Default weight for [`SceneChunk::primitive`].
pub const DEFAULT_CHUNK_WEIGHT: u32 = 1;

/// Scheduling tree for incremental LOD fulfillment.
pub enum LodChunk<P> {
	/// Nested scheduling groups (expanded on demand during fulfill).
	SubChunks(Vec<LodChunk<P>>),
	/// One consume unit with a relative cost heuristic.
	Primitive { weight: u32, payload: P },
	/// Deferred child chunks. `next` builds the next chunk when fulfill needs it.
	///
	/// `remaining_weight` / `remaining_primitives` are the unsettled totals still
	/// expected from `next` (not including chunks already pulled).
	Lazy {
		remaining_weight: u32,
		remaining_primitives: usize,
		next: Box<dyn FnMut() -> Option<LodChunk<P>> + Send + Sync>,
	},
}

/// Main-world spawn tree. Consumed by [`crate::drain_chunk_lod_fulfill`].
pub type SemanticSceneChunk = LodChunk<Box<dyn Scene>>;

/// Compatibility name for [`SemanticSceneChunk`].
pub type SceneChunk = SemanticSceneChunk;

impl<P> LodChunk<P> {
	/// One primitive with an explicit weight.
	pub fn with_payload(weight: u32, payload: P) -> Self {
		Self::Primitive { weight: weight.max(1), payload }
	}

	/// Group child chunks.
	pub fn chunks(chunks: impl IntoIterator<Item = LodChunk<P>>) -> Self {
		Self::SubChunks(chunks.into_iter().collect())
	}

	/// Lazy producer of child chunks (built on fulfill pop, not at begin).
	pub fn lazy(
		remaining_weight: u32,
		remaining_primitives: usize,
		next: impl FnMut() -> Option<LodChunk<P>> + Send + Sync + 'static,
	) -> Self {
		Self::Lazy {
			remaining_weight: remaining_weight.max(1),
			remaining_primitives,
			next: Box::new(next),
		}
	}

	/// Flatten into a FIFO of `(weight, payload)` primitives.
	///
	/// **Eager:** expands [`Self::Lazy`] fully. Prefer [`Self::into_fulfill_queue`]
	/// + [`pull_payload`] for budgeted materialization.
	pub fn into_primitives(self) -> VecDeque<(u32, P)> {
		let mut queue = self.into_fulfill_queue();
		let mut out = VecDeque::new();
		while let Some(prim) = pull_payload(&mut queue) {
			out.push_back(prim);
		}
		out
	}

	/// Queue form that preserves [`Self::Lazy`] / [`Self::SubChunks`] for fulfill.
	pub fn into_fulfill_queue(self) -> VecDeque<LodChunk<P>> {
		match self {
			Self::SubChunks(children) => children.into(),
			other => VecDeque::from([other]),
		}
	}

	/// Total scheduling weight (includes unsettled [`Self::Lazy`] weight).
	pub fn total_weight(&self) -> u32 {
		match self {
			Self::SubChunks(children) => children.iter().map(Self::total_weight).sum(),
			Self::Primitive { weight, .. } => (*weight).max(1),
			Self::Lazy { remaining_weight, .. } => (*remaining_weight).max(1),
		}
	}

	/// Leaf primitive count (includes unsettled [`Self::Lazy`] primitives).
	pub fn total_primitives(&self) -> usize {
		match self {
			Self::SubChunks(children) => children.iter().map(Self::total_primitives).sum(),
			Self::Primitive { .. } => 1,
			Self::Lazy { remaining_primitives, .. } => *remaining_primitives,
		}
	}
}

impl LodChunk<Box<dyn Scene>> {
	/// Single primitive with [`DEFAULT_CHUNK_WEIGHT`].
	pub fn primitive(scene: impl Scene + 'static) -> Self {
		Self::weighted(DEFAULT_CHUNK_WEIGHT, scene)
	}

	/// Single primitive with an explicit weight (relative heuristic).
	pub fn weighted(weight: u32, scene: impl Scene + 'static) -> Self {
		Self::with_payload(weight, Box::new(scene))
	}
}

/// Pull the next payload, expanding [`LodChunk::SubChunks`] / [`LodChunk::Lazy`]
/// at the front as needed.
pub fn pull_payload<P>(queue: &mut VecDeque<LodChunk<P>>) -> Option<(u32, P)> {
	loop {
		let chunk = queue.pop_front()?;
		match chunk {
			LodChunk::Primitive { weight, payload } => {
				return Some((weight.max(1), payload));
			}
			LodChunk::SubChunks(children) => {
				for child in children.into_iter().rev() {
					queue.push_front(child);
				}
			}
			LodChunk::Lazy { remaining_weight, remaining_primitives, mut next } => match next() {
				Some(child) => {
					let child_w = child.total_weight().max(1);
					let child_p = child.total_primitives();
					let rem_w = remaining_weight.saturating_sub(child_w);
					let rem_p = remaining_primitives.saturating_sub(child_p);
					if rem_p > 0 {
						queue.push_front(LodChunk::Lazy {
							remaining_weight: rem_w.max(1),
							remaining_primitives: rem_p,
							next,
						});
					}
					queue.push_front(child);
				}
				None => {}
			},
		}
	}
}

/// Pull the next semantic spawn primitive.
pub fn pull_primitive(queue: &mut VecDeque<SceneChunk>) -> Option<(u32, Box<dyn Scene>)> {
	pull_payload(queue)
}

/// Materialize up to `budget` weight of payloads at the front of `queue`.
///
/// Expands lazy/sub-chunk fronts into [`LodChunk::Primitive`] entries so begin
/// can prefill under its weight budget. Returns weight spent.
pub fn materialize_front<P>(queue: &mut VecDeque<LodChunk<P>>, budget: u32) -> u32 {
	if budget == 0 {
		return 0;
	}
	let mut ready = VecDeque::new();
	let mut spent = 0u32;
	while spent < budget {
		let Some((weight, payload)) = pull_payload(queue) else {
			break;
		};
		spent = spent.saturating_add(weight);
		ready.push_back(LodChunk::Primitive { weight, payload });
	}
	while let Some(chunk) = ready.pop_back() {
		queue.push_front(chunk);
	}
	spent
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
		assert_eq!(chunk.total_primitives(), 3);
		let prims = chunk.into_primitives();
		assert_eq!(prims.len(), 3);
		assert_eq!(prims[0].0, 2);
		assert_eq!(prims[1].0, DEFAULT_CHUNK_WEIGHT);
		assert_eq!(prims[2].0, 5);
	}

	#[test]
	fn lazy_pulls_one_at_a_time() {
		let mut i = 0u32;
		let chunk = SceneChunk::lazy(3, 3, move || {
			if i >= 3 {
				return None;
			}
			i += 1;
			Some(SceneChunk::weighted(1, bevy::scene::SceneFunction(empty_scene)))
		});
		assert_eq!(chunk.total_weight(), 3);
		assert_eq!(chunk.total_primitives(), 3);
		let mut queue = chunk.into_fulfill_queue();
		assert_eq!(materialize_front(&mut queue, 2), 2);
		// Two prefilled primitives + lazy remainder.
		assert_eq!(queue.len(), 3);
		assert!(matches!(queue.front(), Some(SceneChunk::Primitive { .. })));
		let prims: Vec<_> = std::iter::from_fn(|| pull_primitive(&mut queue)).collect();
		assert_eq!(prims.len(), 3);
	}
}
