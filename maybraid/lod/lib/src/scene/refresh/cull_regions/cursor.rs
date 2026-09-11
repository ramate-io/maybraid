//! Round-robin cursor over cull lattice cells.

use bevy::math::IVec3;
use bevy::prelude::*;

/// How often [`super::produce_lod_cull_regions`] advances the shared cursor.
///
/// Default is every frame (tests / playgrounds). Vegetation sets `4` so the
/// 112-cell OpenLattice sweep is ~7.5 s at 60 FPS instead of ~1.9 s, and cull
/// fill is not paid on the skipped ticks.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodCullProduceCadence {
	pub frames_per_emit: u32,
}

impl Default for LodCullProduceCadence {
	fn default() -> Self {
		Self { frames_per_emit: 1 }
	}
}

impl LodCullProduceCadence {
	pub fn every_n_frames(n: u32) -> Self {
		Self { frames_per_emit: n.max(1) }
	}

	/// `true` on the first call and every `frames_per_emit` ticks after that.
	pub fn should_emit(self, frames_since: &mut u32) -> bool {
		let every = self.frames_per_emit.max(1);
		if *frames_since == 0 || *frames_since >= every {
			*frames_since = 1;
			true
		} else {
			*frames_since += 1;
			false
		}
	}
}

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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cadence_emits_first_tick_then_every_n() {
		let cadence = LodCullProduceCadence::every_n_frames(4);
		let mut frames = 0;
		assert!(cadence.should_emit(&mut frames));
		assert!(!cadence.should_emit(&mut frames));
		assert!(!cadence.should_emit(&mut frames));
		assert!(!cadence.should_emit(&mut frames));
		assert!(cadence.should_emit(&mut frames));
	}

	#[test]
	fn default_cadence_emits_every_tick() {
		let cadence = LodCullProduceCadence::default();
		let mut frames = 0;
		assert!(cadence.should_emit(&mut frames));
		assert!(cadence.should_emit(&mut frames));
		assert!(cadence.should_emit(&mut frames));
	}
}
