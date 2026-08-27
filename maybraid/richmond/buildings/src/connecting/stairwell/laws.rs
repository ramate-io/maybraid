//! Shared exclusive-well stair laws.

use richmond_building_components::stairs::StraightStair;

/// Smallest walkable going (meters). Extra laps exist only to stay at or above
/// this when [`MIN_HEADROOM`] still holds.
pub(crate) const MIN_GOING: f32 = 0.25;
/// Smallest rise per lap (meters). A short well keeps one lap and accepts
/// going below [`MIN_GOING`] rather than stacking flights.
pub(crate) const MIN_HEADROOM: f32 = 2.0;
/// Smallest door / rim strip (meters).
pub(crate) const MIN_LANDING: f32 = 0.12;
/// Smallest leftover I run after pads (meters).
pub(crate) const MIN_RUN: f32 = 0.08;

pub(crate) fn resolved_rise(rise: f32) -> f32 {
	rise.max(StraightStair::DEFAULT_TREAD_HEIGHT)
}

pub(crate) fn tread_count(rise: f32) -> u32 {
	(resolved_rise(rise) / StraightStair::DEFAULT_TREAD_HEIGHT).ceil().max(1.0) as u32
}

/// Add laps while going is under [`MIN_GOING`] and the next lap still has
/// [`MIN_HEADROOM`].
pub(crate) fn add_laps_for_going(mut laps: u32, rise: f32, going_of: impl Fn(u32) -> f32) -> u32 {
	while going_of(laps) + 1e-4 < MIN_GOING {
		let next = laps + 1;
		if !headroom_allows(rise, next as f32) {
			break;
		}
		laps = next;
	}
	laps
}

pub(crate) fn headroom_allows(rise: f32, laps: f32) -> bool {
	rise / laps.max(1e-4) + 1e-4 >= MIN_HEADROOM
}
