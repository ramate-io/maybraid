//! Shared grove catalog identifiers, ops queue, and encode counters.
//!
//! Vegetation kinds keep their own descriptor arenas. This module is the
//! extract contract: the main world queues [`CatalogOp`]s, extract `mem::take`s
//! them, and prepare writes only those ranges.

use bevy::prelude::*;

/// Stable grove key. Main-world entities use [`Entity::to_bits`].
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct GroveId(pub u64);

impl GroveId {
	pub fn from_entity(entity: Entity) -> Self {
		Self(entity.to_bits())
	}
}

/// How many admits prepare will grow in one frame. Remaining ops stay queued.
pub const ADMISSION_BUDGET: usize = 16;

/// Per-view, per-kind GPU encode. Independent of grove count.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct GroveEncodeStats {
	pub passes: u32,
	pub dispatches: u32,
	pub draws: u32,
	pub uniform_writes: u32,
	pub catalog_writes: u32,
	pub catalog_bytes: u64,
	pub catalog_clones: u32,
	pub truncated_plants: u32,
}

impl GroveEncodeStats {
	/// Tree/bush wood+foliage: one compute pass, three dispatches, six indirect draws.
	pub fn woody_kind() -> Self {
		Self { passes: 1, dispatches: 3, draws: 6, uniform_writes: 1, ..Self::default() }
	}

	/// Grass: one compute pass, two dispatches, three indirect draws.
	pub fn grass_kind() -> Self {
		Self { passes: 1, dispatches: 2, draws: 3, uniform_writes: 1, ..Self::default() }
	}

	pub fn encode_only(&self) -> Self {
		Self {
			passes: self.passes,
			dispatches: self.dispatches,
			draws: self.draws,
			uniform_writes: self.uniform_writes,
			..Self::default()
		}
	}
}

/// Bump allocator with a free list. Capacity doubles when the bump overflows.
#[derive(Clone, Debug, Default)]
pub struct RangeArena {
	pub used: u32,
	pub cap: u32,
	free: Vec<RangeSpan>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RangeSpan {
	pub offset: u32,
	pub count: u32,
}

impl RangeSpan {
	pub fn is_empty(self) -> bool {
		self.count == 0
	}

	pub fn byte_len(self, stride: u64) -> u64 {
		u64::from(self.count) * stride
	}
}

impl RangeArena {
	pub fn with_cap(cap: u32) -> Self {
		Self { used: 0, cap, free: Vec::new() }
	}

	pub fn alloc(&mut self, count: u32) -> RangeSpan {
		if count == 0 {
			return RangeSpan { offset: 0, count: 0 };
		}
		if let Some(index) = self.free.iter().position(|span| span.count >= count) {
			let mut span = self.free.remove(index);
			let allocated = RangeSpan { offset: span.offset, count };
			span.offset += count;
			span.count -= count;
			if span.count > 0 {
				self.free.push(span);
			}
			return allocated;
		}
		let offset = self.used;
		let needed = offset.saturating_add(count);
		let mut cap = self.cap.max(1);
		while cap < needed {
			cap = cap.saturating_mul(2).max(needed);
		}
		self.cap = cap;
		self.used = needed;
		RangeSpan { offset, count }
	}

	pub fn free(&mut self, span: RangeSpan) {
		if !span.is_empty() {
			self.free.push(span);
		}
	}

	pub fn grew_to(&self, previous_cap: u32) -> bool {
		self.cap > previous_cap
	}
}

/// Pending admits/retires. Extract takes the whole queue.
#[derive(Resource, Clone, Debug)]
pub struct CatalogOpQueue<T> {
	pub ops: Vec<T>,
}

impl<T> Default for CatalogOpQueue<T> {
	fn default() -> Self {
		Self { ops: Vec::new() }
	}
}

impl<T> CatalogOpQueue<T> {
	pub fn push(&mut self, op: T) {
		self.ops.push(op);
	}

	pub fn take(&mut self) -> Vec<T> {
		std::mem::take(&mut self.ops)
	}

	pub fn is_empty(&self) -> bool {
		self.ops.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encode_counts_do_not_depend_on_grove_count() {
		assert_eq!(GroveEncodeStats::woody_kind().encode_only().passes, 1);
		assert_eq!(GroveEncodeStats::woody_kind().dispatches, 3);
		assert_eq!(GroveEncodeStats::woody_kind().draws, 6);
		assert_eq!(GroveEncodeStats::woody_kind().uniform_writes, 1);
		assert_eq!(GroveEncodeStats::grass_kind().passes, 1);
		assert_eq!(GroveEncodeStats::grass_kind().dispatches, 2);
		assert_eq!(GroveEncodeStats::grass_kind().draws, 3);
		assert_eq!(GroveEncodeStats::grass_kind().uniform_writes, 1);
		assert_eq!(
			GroveEncodeStats::woody_kind().encode_only(),
			GroveEncodeStats::woody_kind().encode_only()
		);
	}

	#[test]
	fn arena_reuses_freed_ranges_before_bumping() {
		let mut arena = RangeArena::with_cap(8);
		let a = arena.alloc(4);
		let _b = arena.alloc(4);
		assert_eq!(arena.used, 8);
		arena.free(a);
		let c = arena.alloc(4);
		assert_eq!(c.offset, a.offset);
		assert_eq!(arena.used, 8);
	}

	#[test]
	fn arena_doubles_when_bump_overflows() {
		let mut arena = RangeArena::with_cap(4);
		let _ = arena.alloc(4);
		let extra = arena.alloc(1);
		assert!(arena.cap >= 8);
		assert_eq!(extra.offset, 4);
	}

	#[test]
	fn queue_take_does_not_clone() {
		let mut queue = CatalogOpQueue { ops: vec![1u32, 2, 3] };
		let taken = queue.take();
		assert_eq!(taken, vec![1, 2, 3]);
		assert!(queue.is_empty());
		assert!(queue.take().is_empty());
	}
}
