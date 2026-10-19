//! Authored volume, obstacle, and timeline contract for v1 dust samples.

use bevy::prelude::*;

/// Packed 49-frame, 15 Hz playback window covering `[0, DURATION]`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustPlaybackFrames {
	pub current: u32,
	pub next: u32,
	pub blend: f32,
}

impl DustPlaybackFrames {
	pub fn as_tuple(self) -> (u32, u32, f32) {
		(self.current, self.next, self.blend)
	}
}

/// One authored exclusion box. Half-extents match the playback shader.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustObstacle {
	pub centre: Vec3,
	pub half_extent: Vec3,
}

impl DustObstacle {
	pub fn size(self) -> Vec3 {
		self.half_extent * 2.0
	}
}

/// Spatial box, frame count, and the two diagnostic obstacle volumes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustBounds {
	pub min: Vec3,
	pub max: Vec3,
}

impl DustBounds {
	pub const DURATION: f32 = 3.2;
	pub const FRAME_RATE: f32 = 15.0;
	pub const FRAME_COUNT: u32 = 49;
	pub const LAST_FRAME: u32 = 48;
	pub const GROUND_FALLOFF: f32 = 0.025;
	pub const CAPTURE_WIDTH_M: f32 = 7.8;
	pub const QUAD_CENTRE: Vec3 = Vec3::new(0.0, 0.55, 0.0);

	pub const V1: Self = Self { min: Vec3::new(-3.2, 0.0, -3.2), max: Vec3::new(3.2, 3.2, 3.2) };

	pub const WALL: DustObstacle = DustObstacle {
		centre: Vec3::new(0.65, 0.7, -0.3),
		half_extent: Vec3::new(0.15, 0.7, 1.25),
	};

	pub const BOX: DustObstacle =
		DustObstacle { centre: Vec3::new(-1.0, 0.4, -0.7), half_extent: Vec3::new(0.4, 0.4, 0.4) };

	pub const OBSTACLES: [DustObstacle; 2] = [Self::WALL, Self::BOX];

	pub fn size(self) -> Vec3 {
		self.max - self.min
	}

	pub fn contains_time(t: f32) -> bool {
		t.is_finite() && (0.0..=Self::DURATION).contains(&t)
	}

	pub fn clamp_time(t: f32) -> f32 {
		t.clamp(0.0, Self::DURATION)
	}

	/// Source-frame pair and blend for a simulation timestamp.
	pub fn frames(t: f32) -> DustPlaybackFrames {
		let f = Self::clamp_time(t) * Self::FRAME_RATE;
		let current = (f.floor() as u32).min(Self::LAST_FRAME);
		DustPlaybackFrames {
			current,
			next: (current + 1).min(Self::LAST_FRAME),
			blend: f - current as f32,
		}
	}
}

impl Default for DustBounds {
	fn default() -> Self {
		Self::V1
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn time_clamps_and_interpolates() {
		assert_eq!(DustBounds::frames(-1.0).as_tuple(), (0, 1, 0.0));
		assert_eq!(DustBounds::frames(0.1).as_tuple(), (1, 2, 0.5));
		assert_eq!(DustBounds::frames(9.0).as_tuple(), (48, 48, 0.0));
		assert!(DustBounds::contains_time(0.7));
		assert!(!DustBounds::contains_time(3.21));
	}

	#[test]
	fn obstacle_sizes_match_authored_geometry() {
		assert_eq!(DustBounds::WALL.size(), Vec3::new(0.3, 1.4, 2.5));
		assert_eq!(DustBounds::BOX.size(), Vec3::new(0.8, 0.8, 0.8));
		assert_eq!(DustBounds::V1.size(), Vec3::new(6.4, 3.2, 6.4));
	}
}
