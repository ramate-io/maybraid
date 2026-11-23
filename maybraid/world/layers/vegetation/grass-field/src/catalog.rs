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

/// wgpu / Metal maximum workgroups on one dispatch axis.
pub const MAX_COMPUTE_WORKGROUPS_PER_DIM: u32 = 65535;

/// One `dispatch_workgroups` covering `item_count` invocations of `workgroup_size`.
///
/// A 1D X dispatch of `ceil(count / 64)` overflows the device limit once the
/// catalog grows past `65535 * 64` descriptors. Tile extra groups onto Y so
/// every admitted item still runs, while encode stays one dispatch per stage.
pub fn select_dispatch_size(item_count: u32, workgroup_size: u32, max_workgroups: u32) -> UVec3 {
	let workgroup_size = workgroup_size.max(1);
	let max_workgroups = max_workgroups.max(1);
	let groups = item_count.div_ceil(workgroup_size).max(1);
	if groups <= max_workgroups {
		return UVec3::new(groups, 1, 1);
	}
	let y = groups.div_ceil(max_workgroups).max(1);
	let x = groups.div_ceil(y).min(max_workgroups).max(1);
	UVec3::new(x, y, 1)
}

/// Linear catalog index matching `@workgroup_size(workgroup_size, 1, 1)` plus a
/// possibly 2D workgroup grid. `gid.x` already includes the local invocation.
pub fn select_item_index(gid: UVec3, num_workgroups: UVec3, workgroup_size: u32) -> u32 {
	gid.x + gid.y * num_workgroups.x * workgroup_size.max(1)
}

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
	/// Tree/bush wood+foliage: grove cull, write, wood, foliage, finalize.
	pub fn woody_kind() -> Self {
		Self { passes: 1, dispatches: 5, draws: 6, uniform_writes: 1, ..Self::default() }
	}

	/// Grass: one compute pass, four dispatches (grove cull + write + select + finalize).
	pub fn grass_kind() -> Self {
		Self { passes: 1, dispatches: 4, draws: 3, uniform_writes: 1, ..Self::default() }
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

/// One catalog op that can be admitted or retired under [`ADMISSION_BUDGET`].
pub trait GroveCatalogOp {
	fn grove_id(&self) -> GroveId;
	fn is_retire(&self) -> bool;
}

/// Applied vs deferred ops from one budgeted catalog pass.
///
/// Retires always apply, even after the admission budget is exhausted. Deferred
/// admits stay in [`Self::leftover`] unless a later retire of the same grove
/// cancels them. [`Self::applied`] keeps the original relative order so extract
/// can replay the same sequence on the render catalog.
#[derive(Debug)]
pub struct CatalogApply<T> {
	pub applied: Vec<T>,
	pub leftover: Vec<T>,
}

impl<T: GroveCatalogOp> CatalogApply<T> {
	pub fn budgeted(
		ops: impl IntoIterator<Item = T>,
		budget: usize,
		mut apply_one: impl FnMut(&T),
	) -> Self {
		let mut leftover: Vec<T> = Vec::new();
		let mut applied: Vec<T> = Vec::new();
		let mut admits = 0usize;
		for op in ops {
			if op.is_retire() {
				apply_one(&op);
				let id = op.grove_id();
				leftover.retain(|pending: &T| pending.is_retire() || pending.grove_id() != id);
				applied.push(op);
				continue;
			}
			if admits >= budget {
				leftover.push(op);
				continue;
			}
			apply_one(&op);
			admits += 1;
			applied.push(op);
		}
		Self { applied, leftover }
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
		assert_eq!(GroveEncodeStats::woody_kind().dispatches, 5);
		assert_eq!(GroveEncodeStats::woody_kind().draws, 6);
		assert_eq!(GroveEncodeStats::woody_kind().uniform_writes, 1);
		assert_eq!(GroveEncodeStats::grass_kind().passes, 1);
		assert_eq!(GroveEncodeStats::grass_kind().dispatches, 4);
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

	#[test]
	fn select_dispatch_keeps_1d_under_the_device_limit() {
		let size = select_dispatch_size(64 * 100, 64, MAX_COMPUTE_WORKGROUPS_PER_DIM);
		assert_eq!(size, UVec3::new(100, 1, 1));
		assert_eq!(select_item_index(UVec3::new(63, 0, 0), size, 64), 63);
	}

	#[test]
	fn select_dispatch_tiles_the_native_bush_overflow() {
		// Metal smoke on f2e9822: ceil(leaf_count / 64) = 66744 > 65535.
		let groups = 66_744u32;
		let items = groups * 64;
		let size = select_dispatch_size(items, 64, MAX_COMPUTE_WORKGROUPS_PER_DIM);
		assert!(size.x <= MAX_COMPUTE_WORKGROUPS_PER_DIM, "{}", size.x);
		assert!(size.y <= MAX_COMPUTE_WORKGROUPS_PER_DIM, "{}", size.y);
		assert_eq!(size.z, 1);
		assert!(size.y > 1, "1D X would still overflow");
		let covered = u64::from(size.x) * u64::from(size.y) * 64;
		assert!(covered >= u64::from(items), "{covered} < {items}");
		let last = select_item_index(UVec3::new(size.x * 64 - 1, size.y - 1, 0), size, 64);
		assert!(last >= items - 1, "{last}");
		assert_eq!(
			select_item_index(UVec3::new(0, 1, 0), size, 64),
			size.x * 64,
			"Y tile must continue the catalog without gaps"
		);
	}

	#[derive(Clone, Debug, PartialEq, Eq)]
	enum FakeOp {
		Admit(u64),
		Retire(u64),
	}

	impl GroveCatalogOp for FakeOp {
		fn grove_id(&self) -> GroveId {
			match *self {
				Self::Admit(id) | Self::Retire(id) => GroveId(id),
			}
		}

		fn is_retire(&self) -> bool {
			matches!(self, Self::Retire(_))
		}
	}

	#[test]
	fn budgeted_apply_forwards_retire_after_deferred_admit() {
		let mut live = vec![99u64];
		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64 + 1).map(FakeOp::Admit).collect();
		batch.push(FakeOp::Retire(99));
		let CatalogApply { applied, leftover } =
			CatalogApply::budgeted(batch, ADMISSION_BUDGET, |op| match *op {
				FakeOp::Admit(id) => live.push(id),
				FakeOp::Retire(id) => live.retain(|live_id| *live_id != id),
			});
		assert_eq!(leftover, [FakeOp::Admit(ADMISSION_BUDGET as u64 + 1)]);
		assert!(applied.contains(&FakeOp::Retire(99)));
		assert!(!applied.contains(&FakeOp::Admit(ADMISSION_BUDGET as u64 + 1)));
		assert!(!live.contains(&99));
		assert!(!live.contains(&(ADMISSION_BUDGET as u64 + 1)));
		for id in 1..=ADMISSION_BUDGET as u64 {
			assert!(live.contains(&id), "{id}");
		}
	}

	#[test]
	fn budgeted_apply_cancels_pending_admit_on_same_id_retire() {
		let mut live = Vec::new();
		let mut batch: Vec<_> = (1..=ADMISSION_BUDGET as u64).map(FakeOp::Admit).collect();
		batch.push(FakeOp::Admit(100));
		batch.push(FakeOp::Retire(100));
		let CatalogApply { applied, leftover } =
			CatalogApply::budgeted(batch, ADMISSION_BUDGET, |op| match *op {
				FakeOp::Admit(id) => live.push(id),
				FakeOp::Retire(id) => live.retain(|live_id| *live_id != id),
			});
		assert!(leftover.is_empty());
		assert!(applied.contains(&FakeOp::Retire(100)));
		assert!(!applied.contains(&FakeOp::Admit(100)));
		assert!(!live.contains(&100));
	}
}
