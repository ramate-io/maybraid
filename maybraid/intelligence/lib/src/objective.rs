//! Movement goals. Higher-order systems write these; this crate does not track entities.

use crate::location::MovementLocation;

/// What the mover is trying to achieve relative to a [`MovementLocation`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MovementObjective {
	/// Arrive inside the location disk.
	Reach(MovementLocation),
	/// Arrive on the disk boundary (just outside the interior).
	EdgeOf(MovementLocation),
	/// Leave the disk, from inside or through it.
	FleeFrom(MovementLocation),
	/// Take a position that conceals from, and observes, the location.
	VantageOn {
		location: MovementLocation,
		/// Value of concealing the body from the location.
		hide_weight: f32,
		/// Value of a sightline to the location.
		sightline_weight: f32,
	},
}

impl MovementObjective {
	pub fn location(self) -> MovementLocation {
		match self {
			Self::Reach(location) | Self::EdgeOf(location) | Self::FleeFrom(location) => location,
			Self::VantageOn { location, .. } => location,
		}
	}

	pub fn hide_weight(self) -> f32 {
		match self {
			Self::VantageOn { hide_weight, .. } => hide_weight,
			_ => 0.0,
		}
	}

	pub fn sightline_weight(self) -> f32 {
		match self {
			Self::VantageOn { sightline_weight, .. } => sightline_weight,
			_ => 0.0,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::*;

	#[test]
	fn vantage_on_exposes_weights() -> anyhow::Result<()> {
		let objective = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::ZERO, 1.0),
			hide_weight: 2.0,
			sightline_weight: 3.0,
		};
		assert_eq!(objective.hide_weight(), 2.0);
		assert_eq!(objective.sightline_weight(), 3.0);
		assert_eq!(objective.location().radius, 1.0);
		Ok(())
	}
}
