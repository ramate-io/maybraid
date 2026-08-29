//! Stick / trigger helpers.

use bevy::prelude::*;

/// Radial / 1D deadzone that rescales the remainder to the full range.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Deadzone(pub f32);

impl Deadzone {
	pub fn apply(self, value: f32) -> f32 {
		let dead = self.0.max(0.0);
		if value.abs() <= dead {
			return 0.0;
		}
		let sign = value.signum();
		let denom = 1.0 - dead;
		if denom <= f32::EPSILON {
			return 0.0;
		}
		sign * (value.abs() - dead) / denom
	}

	pub fn apply_vec2(self, value: Vec2) -> Vec2 {
		let dead = self.0.max(0.0);
		let length = value.length();
		if length <= dead {
			return Vec2::ZERO;
		}
		let denom = 1.0 - dead;
		if denom <= f32::EPSILON {
			return Vec2::ZERO;
		}
		value * ((length - dead) / denom / length)
	}
}

/// Dominant cardinal from a stick or dpad, or `None` under `threshold`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cardinal {
	Up,
	Down,
	Left,
	Right,
}

impl Cardinal {
	pub fn from_stick(stick: Vec2, threshold: f32) -> Option<Self> {
		if stick.length() < threshold {
			return None;
		}
		if stick.x.abs() > stick.y.abs() {
			if stick.x > 0.0 {
				Some(Self::Right)
			} else {
				Some(Self::Left)
			}
		} else if stick.y > 0.0 {
			Some(Self::Up)
		} else {
			Some(Self::Down)
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn deadzone_zeros_interior() -> anyhow::Result<()> {
		let zone = Deadzone(0.2);
		assert_eq!(zone.apply(0.1), 0.0);
		assert!((zone.apply(1.0) - 1.0).abs() < 1e-5);
		assert!((zone.apply(0.6) - 0.5).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn stick_deadzone_is_radial() -> anyhow::Result<()> {
		let zone = Deadzone(0.25);
		assert_eq!(zone.apply_vec2(Vec2::new(0.1, 0.1)), Vec2::ZERO);
		let out = zone.apply_vec2(Vec2::new(1.0, 0.0));
		assert!((out.x - 1.0).abs() < 1e-5);
		assert!(out.y.abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn cardinal_picks_dominant_axis() -> anyhow::Result<()> {
		assert_eq!(Cardinal::from_stick(Vec2::new(0.1, 0.0), 0.25), None);
		assert_eq!(Cardinal::from_stick(Vec2::new(0.8, 0.2), 0.25), Some(Cardinal::Right));
		assert_eq!(Cardinal::from_stick(Vec2::new(-0.1, 0.9), 0.25), Some(Cardinal::Up));
		Ok(())
	}
}
