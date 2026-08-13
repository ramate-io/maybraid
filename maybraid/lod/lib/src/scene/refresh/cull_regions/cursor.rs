//! Round-robin cursor over cull lattice cells.

use bevy::math::IVec3;
use bevy::prelude::*;

/// Stable RR state for [`super::LodCullRegions`] producers (e.g. [`super::OpenLattice`]).
///
/// - **`cells`** is replaced when the driver anchor cell changes.
/// - **`next`** wraps forever — production does not exhaust-and-stop.
#[derive(Resource, Debug, Clone)]
pub struct LodCullRegionCursor {
	pub cells: Vec<IVec3>,
	pub next: u32,
	pub anchor_cell: Option<IVec3>,
	/// How many cell AABBs to emit per production tick (default 1).
	pub regions_per_tick: u32,
}

impl Default for LodCullRegionCursor {
	fn default() -> Self {
		Self { cells: Vec::new(), next: 0, anchor_cell: None, regions_per_tick: 1 }
	}
}

impl LodCullRegionCursor {
	pub fn with_regions_per_tick(mut self, n: u32) -> Self {
		self.regions_per_tick = n.max(1);
		self
	}

	/// Drop the cached cell list so the next produce rebuilds (e.g. lattice params changed).
	pub fn invalidate_cells(&mut self) {
		self.anchor_cell = None;
	}

	/// Rebuild the cell list when `anchor` changes; otherwise keep the round-robin cursor.
	pub fn ensure_cells(&mut self, anchor: IVec3, enumerate: impl FnOnce() -> Vec<IVec3>) {
		if self.anchor_cell == Some(anchor) {
			return;
		}
		self.anchor_cell = Some(anchor);
		self.cells = enumerate();
		self.next = 0;
	}

	/// Take up to [`Self::regions_per_tick`] cells, advancing with wrap.
	pub fn take_cells(&mut self) -> Vec<IVec3> {
		let n = self.cells.len();
		if n == 0 {
			return Vec::new();
		}
		let count = (self.regions_per_tick as usize).max(1).min(n);
		let mut out = Vec::with_capacity(count);
		for _ in 0..count {
			let i = (self.next as usize) % n;
			out.push(self.cells[i]);
			self.next = self.next.wrapping_add(1);
		}
		out
	}
}
