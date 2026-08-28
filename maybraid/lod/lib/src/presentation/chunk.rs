//! Incremental presentation composition ([`PresentationChunk`]).
//!
//! Same scheduling shape as [`crate::LodChunk`], without LOD bands. A type
//! expands into lower-order constituents; fulfill drains weighted primitives
//! under a per-frame budget.

use std::collections::VecDeque;

/// Default weight for [`PresentationChunk::primitive`].
pub const DEFAULT_PRESENTATION_WEIGHT: u32 = 1;

/// Scheduling tree for incremental presentation (no banding).
pub enum PresentationChunk<C> {
	/// Nested scheduling groups (expanded on demand during fulfill).
	SubChunks(Vec<PresentationChunk<C>>),
	/// One spawn unit with a relative cost heuristic.
	Primitive { weight: u32, constituent: C },
	/// Deferred child chunks. `next` builds the next chunk when fulfill needs it.
	Lazy {
		remaining_weight: u32,
		remaining_primitives: usize,
		next: Box<dyn FnMut() -> Option<PresentationChunk<C>> + Send + Sync>,
	},
}

impl<C> PresentationChunk<C> {
	/// Single primitive with [`DEFAULT_PRESENTATION_WEIGHT`].
	pub fn primitive(constituent: C) -> Self {
		Self::weighted(DEFAULT_PRESENTATION_WEIGHT, constituent)
	}

	/// Single primitive with an explicit weight.
	pub fn weighted(weight: u32, constituent: C) -> Self {
		Self::Primitive { weight: weight.max(1), constituent }
	}

	/// Group child chunks.
	pub fn chunks(chunks: impl IntoIterator<Item = PresentationChunk<C>>) -> Self {
		Self::SubChunks(chunks.into_iter().collect())
	}

	/// Lazy producer of child chunks (built on fulfill pop, not at begin).
	pub fn lazy(
		remaining_weight: u32,
		remaining_primitives: usize,
		next: impl FnMut() -> Option<PresentationChunk<C>> + Send + Sync + 'static,
	) -> Self {
		Self::Lazy {
			remaining_weight: remaining_weight.max(1),
			remaining_primitives,
			next: Box::new(next),
		}
	}

	/// Queue form that preserves [`Self::Lazy`] / [`Self::SubChunks`] for fulfill.
	pub fn into_fulfill_queue(self) -> VecDeque<PresentationChunk<C>> {
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

/// Pull the next spawnable primitive, expanding groups / lazy fronts as needed.
pub fn pull_constituent<C>(queue: &mut VecDeque<PresentationChunk<C>>) -> Option<(u32, C)> {
	loop {
		let chunk = queue.pop_front()?;
		match chunk {
			PresentationChunk::Primitive { weight, constituent } => {
				return Some((weight.max(1), constituent));
			}
			PresentationChunk::SubChunks(children) => {
				for child in children.into_iter().rev() {
					queue.push_front(child);
				}
			}
			PresentationChunk::Lazy { remaining_weight, remaining_primitives, mut next } => {
				match next() {
					Some(child) => {
						let child_w = child.total_weight().max(1);
						let child_p = child.total_primitives();
						let rem_w = remaining_weight.saturating_sub(child_w);
						let rem_p = remaining_primitives.saturating_sub(child_p);
						if rem_p > 0 {
							queue.push_front(PresentationChunk::Lazy {
								remaining_weight: rem_w.max(1),
								remaining_primitives: rem_p,
								next,
							});
						}
						queue.push_front(child);
					}
					None => {}
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flatten_preserves_order_and_weights() {
		let chunk = PresentationChunk::chunks([
			PresentationChunk::weighted(2, "a"),
			PresentationChunk::chunks([PresentationChunk::primitive("b")]),
			PresentationChunk::weighted(5, "c"),
		]);
		assert_eq!(chunk.total_weight(), 8);
		assert_eq!(chunk.total_primitives(), 3);
		let mut queue = chunk.into_fulfill_queue();
		let mut out = Vec::new();
		while let Some(prim) = pull_constituent(&mut queue) {
			out.push(prim);
		}
		assert_eq!(out, vec![(2, "a"), (1, "b"), (5, "c")]);
	}

	#[test]
	fn lazy_pulls_one_at_a_time() {
		let mut i = 0u32;
		let chunk = PresentationChunk::lazy(3, 3, move || {
			if i >= 3 {
				return None;
			}
			i += 1;
			Some(PresentationChunk::weighted(1, i))
		});
		let mut queue = chunk.into_fulfill_queue();
		assert_eq!(pull_constituent(&mut queue).map(|p| p.1), Some(1));
		assert_eq!(pull_constituent(&mut queue).map(|p| p.1), Some(2));
		assert_eq!(pull_constituent(&mut queue).map(|p| p.1), Some(3));
		assert!(pull_constituent(&mut queue).is_none());
	}
}
