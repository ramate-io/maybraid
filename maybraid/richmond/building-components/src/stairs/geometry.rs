//! Continuous stair geometry.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stair {
	Spiral(SpiralStair),
	Straight(StraightStair),
}

impl Stair {
	pub fn spiral() -> Self {
		Self::Spiral(SpiralStair::default())
	}

	/// Circular run of treads that rises `height` meters around `radius`.
	pub fn spiral_run(height: f32, radius: f32, tread_width: f32, tread_depth: f32) -> Self {
		Self::Spiral(SpiralStair {
			height: height.max(1e-4),
			radius: radius.max(1e-4),
			tread_width: tread_width.max(1e-4),
			tread_depth: tread_depth.max(1e-4),
			tread_height: SpiralStair::DEFAULT_TREAD_HEIGHT,
			turns: 1.0,
		})
	}

	pub fn straight() -> Self {
		Self::Straight(StraightStair)
	}
}

/// Spiral / circular stair parameterized for tread tessellation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiralStair {
	/// Total rise to the next level (meters).
	pub height: f32,
	/// Centerline radius of the tread run.
	pub radius: f32,
	/// Radial tread width (world meters).
	pub tread_width: f32,
	/// Tangential tread depth / run (world meters).
	pub tread_depth: f32,
	/// Target rise per tread (~0.18 m per README).
	pub tread_height: f32,
	/// Number of full turns over `height` (1.0 = one revolution).
	pub turns: f32,
}

impl SpiralStair {
	pub const DEFAULT_TREAD_HEIGHT: f32 = 0.18;

	pub fn tread_count(self) -> u32 {
		let h = self.tread_height.max(1e-4);
		(self.height / h).ceil().max(1.0) as u32
	}

	/// Exact rise per tread so the run lands on `height`.
	pub fn rise_per_tread(self) -> f32 {
		self.height / self.tread_count() as f32
	}
}

impl Default for SpiralStair {
	fn default() -> Self {
		Self {
			height: 3.0,
			radius: 1.0,
			tread_width: 0.5,
			tread_depth: 0.35,
			tread_height: Self::DEFAULT_TREAD_HEIGHT,
			turns: 1.0,
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StraightStair;
