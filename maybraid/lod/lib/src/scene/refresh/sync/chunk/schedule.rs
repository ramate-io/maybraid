//! Presence / Desired / Active admission and drain ordering.
//!
//! Under saturation, budget is split ~¼ Presence / ~⅜ Desired / ~⅜ Active.
//! Drain ranks by `(parent_desired, self_level)` High→… within each class
//! (missing parent counts as High). Frame parity rotates which class runs first;
//! leftovers cascade into the remaining classes.
//! Round-robin cursors avoid stable ECS-order starvation within a tuple band.

use bevy::prelude::*;

use crate::scene::level::LodSceneLevel;

use super::types::{
	FulfillClass, LodChunkBeginClock, LodChunkBudgetClock, LodChunkDrainCursor,
	LodChunkFulfillBudget, LOD_CHUNK_TUPLE_BAND_COUNT,
};

/// Named drain / begin band (near → far).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LevelBand {
	High,
	Medium,
	Low,
	UltraLow,
	Other,
}

impl LevelBand {
	pub(super) const COUNT: usize = 5;

	pub(super) fn from_level(level: LodSceneLevel) -> Self {
		match level {
			LodSceneLevel::High => Self::High,
			LodSceneLevel::Medium => Self::Medium,
			LodSceneLevel::Low => Self::Low,
			LodSceneLevel::UltraLow => Self::UltraLow,
			LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => Self::Other,
		}
	}

	pub(super) fn index(self) -> usize {
		match self {
			Self::High => 0,
			Self::Medium => 1,
			Self::Low => 2,
			Self::UltraLow => 3,
			Self::Other => 4,
		}
	}

	/// Lexicographic `(parent, self)` rank: `(High, High) = 0` … far/far last.
	pub(super) fn tuple_rank(parent: Self, self_band: Self) -> usize {
		debug_assert_eq!(Self::COUNT * Self::COUNT, LOD_CHUNK_TUPLE_BAND_COUNT);
		parent.index() * Self::COUNT + self_band.index()
	}

	/// High / Medium — preferred in the Desired begin pass before Low / UltraLow.
	pub(super) fn is_near(self) -> bool {
		matches!(self, Self::High | Self::Medium)
	}
}

/// Split total into Presence / Desired / Active (~¼ / ~⅜ / ~⅜).
pub(super) fn split_presence_desired_active(total: u32) -> (u32, u32, u32) {
	if total == 0 {
		return (0, 0, 0);
	}
	let active = (total * 3) / 8;
	let desired = (total * 3) / 8;
	let presence = total - active - desired;
	(presence, desired, active)
}

/// Class drain / begin order for this frame (`frame % 3`).
pub(super) fn class_order(frame: u64) -> [FulfillClass; 3] {
	match frame % 3 {
		0 => [FulfillClass::Presence, FulfillClass::Desired, FulfillClass::Active],
		1 => [FulfillClass::Desired, FulfillClass::Active, FulfillClass::Presence],
		_ => [FulfillClass::Active, FulfillClass::Presence, FulfillClass::Desired],
	}
}

/// Reset spawn/cull/begin clocks and advance drain frame parity.
pub fn reset_lod_chunk_budget(
	budget: Res<LodChunkFulfillBudget>,
	mut spawn_clock: ResMut<LodChunkBudgetClock>,
	mut begin_clock: ResMut<LodChunkBeginClock>,
	mut drain_cursor: ResMut<LodChunkDrainCursor>,
) {
	spawn_clock.spawn_remaining = budget.spawn_weights_per_frame;
	spawn_clock.cull_remaining = budget.cull_weights_per_frame;

	let (presence, desired, active) = split_presence_desired_active(budget.begins_per_frame);
	drain_cursor.frame = drain_cursor.frame.wrapping_add(1);
	*begin_clock = LodChunkBeginClock {
		presence_remaining: presence,
		desired_remaining: desired,
		active_remaining: active,
		first_class: class_order(drain_cursor.frame)[0],
	};
}

/// Try to admit one begin into `class`. Returns false if that class's quota is empty.
pub(super) fn admit_begin(clock: &mut LodChunkBeginClock, class: FulfillClass) -> bool {
	let slot = match class {
		FulfillClass::Presence => &mut clock.presence_remaining,
		FulfillClass::Desired => &mut clock.desired_remaining,
		FulfillClass::Active => &mut clock.active_remaining,
	};
	if *slot == 0 {
		return false;
	}
	*slot -= 1;
	true
}

/// Round-robin walk of `items`, invoking `visit` until it returns `false` (budget empty).
pub(super) fn for_each_rr<T>(items: &[T], cursor: &mut u32, mut visit: impl FnMut(&T) -> bool) {
	let n = items.len();
	if n == 0 {
		return;
	}
	let start = (*cursor as usize) % n;
	*cursor = cursor.wrapping_add(1);
	for i in 0..n {
		if !visit(&items[(start + i) % n]) {
			break;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn split_three_way() {
		assert_eq!(split_presence_desired_active(0), (0, 0, 0));
		assert_eq!(split_presence_desired_active(8), (2, 3, 3));
		assert_eq!(split_presence_desired_active(1), (1, 0, 0));
		assert_eq!(split_presence_desired_active(48), (12, 18, 18));
	}

	#[test]
	fn tuple_rank_high_high_first() {
		assert_eq!(
			LevelBand::tuple_rank(LevelBand::High, LevelBand::High),
			0
		);
		assert_eq!(
			LevelBand::tuple_rank(LevelBand::High, LevelBand::Medium),
			1
		);
		assert_eq!(
			LevelBand::tuple_rank(LevelBand::Medium, LevelBand::High),
			LevelBand::COUNT
		);
		assert!(
			LevelBand::tuple_rank(LevelBand::High, LevelBand::UltraLow)
				< LevelBand::tuple_rank(LevelBand::Medium, LevelBand::High)
		);
	}
}
