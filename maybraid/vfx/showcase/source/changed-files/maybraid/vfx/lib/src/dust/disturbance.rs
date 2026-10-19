//! Local, stateless influence used by the dust playback shader.

use bevy::prelude::*;

/// Smoothed mover used to clear and shift nearby coverage/density.
///
/// The GPU path stays stateless: a radial weight, a ≤ 0.16 m coordinate shift,
/// and no pileup or wake. These fields pack into the playback uniform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustDisturbance {
	pub position: Vec3,
	pub velocity: Vec3,
	pub strength: f32,
	pub radius: f32,
}

impl DustDisturbance {
	pub const RADIUS: f32 = 0.85;
	pub const MAX_SPEED: f32 = 4.0;
	pub const MAX_VELOCITY: f32 = 3.0;
	pub const SMOOTHING: f32 = 0.18;
	pub const SHIFT_SCALE: f32 = 0.055;
	pub const CLEAR_SCALE: f32 = 0.58;

	pub fn idle_at(position: Vec3) -> Self {
		Self { position, velocity: Vec3::ZERO, strength: 0.0, radius: Self::RADIUS }
	}

	pub fn target_strength(speed: f32, interact: bool) -> f32 {
		if interact {
			speed.min(Self::MAX_SPEED) / Self::MAX_SPEED
		} else {
			0.0
		}
	}

	pub fn smooth_strength(current: f32, target: f32, dt: f32) -> f32 {
		current + (target - current) * (1.0 - (-dt / Self::SMOOTHING).exp())
	}

	pub fn packed_influence(self) -> Vec4 {
		self.position.extend(self.strength)
	}

	pub fn packed_velocity(self) -> Vec4 {
		self.velocity.clamp_length_max(Self::MAX_VELOCITY).extend(self.radius)
	}
}

impl Default for DustDisturbance {
	fn default() -> Self {
		Self::idle_at(Vec3::ZERO)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn interaction_scales_and_clamps_speed() {
		assert_eq!(DustDisturbance::target_strength(2.0, false), 0.0);
		assert!((DustDisturbance::target_strength(2.0, true) - 0.5).abs() < 1e-6);
		assert_eq!(DustDisturbance::target_strength(8.0, true), 1.0);
	}

	#[test]
	fn packing_clamps_velocity_and_keeps_radius() {
		let field = DustDisturbance {
			position: Vec3::new(1.0, 0.4, 0.1),
			velocity: Vec3::X * 9.0,
			strength: 0.25,
			radius: DustDisturbance::RADIUS,
		};
		assert_eq!(field.packed_influence(), Vec4::new(1.0, 0.4, 0.1, 0.25));
		let packed = field.packed_velocity();
		assert!((packed.truncate().length() - DustDisturbance::MAX_VELOCITY).abs() < 1e-5);
		assert!((packed.w - DustDisturbance::RADIUS).abs() < 1e-6);
	}
}
