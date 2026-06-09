//! Grove cell selection outcomes ([RFC-183 3.4.2.2]).

use bevy_math::Vec3;

/// Result of running the full selection pipeline on one grove cell.
#[derive(Debug, Clone, PartialEq)]
pub enum GroveCellOutcome<V> {
	Placed {
		variant: V,
		position: Vec3,
		scale: f32,
	},
	/// Explicit `None` bucket won first-fit at this candidate point. The position is the
	/// evaluated placement (cell origin + offset) so empty outcomes stay addressable in space
	/// and remain stable across chunk reloads, matching RFC-170-style ownership from the parent cell.
	Empty {
		position: Vec3,
	},
	/// Every bucket failed placement constraints at this candidate point. The position records
	/// where validation ran so callers can debug terrain mismatch without inventing a fallback
	/// location (which would introduce flicker or migration artifacts).
	Rejected {
		position: Vec3,
	},
}
