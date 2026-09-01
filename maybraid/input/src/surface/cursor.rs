//! Bounded cursor driven by move / look / dpad.

use bevy::prelude::*;

use crate::config::VirtualPadConfig;
use crate::pad::VirtualPad;

#[derive(Resource, Clone, Debug)]
pub struct PadCursor {
	pub position: Vec2,
	pub bounds: Option<Rect>,
}

impl Default for PadCursor {
	fn default() -> Self {
		Self { position: Vec2::ZERO, bounds: None }
	}
}

impl PadCursor {
	pub fn with_bounds(bounds: Rect) -> Self {
		Self { position: bounds.center(), bounds: Some(bounds) }
	}

	pub fn clamp_in_bounds(&mut self) {
		let Some(bounds) = self.bounds else {
			return;
		};
		self.position = self.position.clamp(bounds.min, bounds.max);
	}

	pub fn integrate(&mut self, pad: &VirtualPad, speed: f32, dt: f32) {
		let stick = if pad.move_stick.length_squared() > pad.dpad.length_squared() {
			pad.move_stick
		} else {
			pad.dpad
		};
		self.position += stick * speed * dt;
		self.clamp_in_bounds();
	}
}

pub fn integrate_cursor(
	pad: Res<VirtualPad>,
	config: Res<VirtualPadConfig>,
	time: Res<Time>,
	mut cursor: ResMut<PadCursor>,
) {
	cursor.integrate(&pad, config.cursor_speed, time.delta_secs());
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clamp_keeps_point_inside() -> anyhow::Result<()> {
		let mut cursor = PadCursor {
			position: Vec2::new(-10.0, 50.0),
			bounds: Some(Rect::from_corners(Vec2::ZERO, Vec2::new(8.0, 8.0))),
		};
		cursor.clamp_in_bounds();
		assert_eq!(cursor.position, Vec2::new(0.0, 8.0));
		Ok(())
	}

	#[test]
	fn integrate_moves_and_clamps() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.move_stick = Vec2::X;
		let mut cursor = PadCursor::with_bounds(Rect::from_corners(Vec2::ZERO, Vec2::splat(10.0)));
		cursor.position = Vec2::new(9.0, 5.0);
		cursor.integrate(&pad, 100.0, 1.0);
		assert_eq!(cursor.position, Vec2::new(10.0, 5.0));
		Ok(())
	}
}
