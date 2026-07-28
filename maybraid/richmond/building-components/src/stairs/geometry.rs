//! Continuous stair geometry.

#[derive(Debug, Clone, PartialEq)]
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
			tread_tops: Vec::new(),
		})
	}

	/// Circular run with explicit local tread-top \(Y\) bindings (ascending).
	pub fn spiral_fitted(
		radius: f32,
		tread_width: f32,
		tread_depth: f32,
		tread_tops: Vec<f32>,
		turns: f32,
	) -> Self {
		let tops = normalize_tops(tread_tops);
		let height = tops.last().copied().unwrap_or(SpiralStair::DEFAULT_TREAD_HEIGHT);
		Self::Spiral(SpiralStair {
			height: height.max(1e-4),
			radius: radius.max(1e-4),
			tread_width: tread_width.max(1e-4),
			tread_depth: tread_depth.max(1e-4),
			tread_height: SpiralStair::DEFAULT_TREAD_HEIGHT,
			turns: turns.max(1e-4),
			tread_tops: tops,
		})
	}

	pub fn straight() -> Self {
		Self::Straight(StraightStair)
	}
}

/// Spiral / circular stair parameterized for tread tessellation.
#[derive(Debug, Clone, PartialEq)]
pub struct SpiralStair {
	/// Total rise to the next level (meters). Ignored when [`Self::tread_tops`] is non-empty
	/// except as a fallback height.
	pub height: f32,
	/// Centerline radius of the tread run.
	pub radius: f32,
	/// Radial tread width (world meters).
	pub tread_width: f32,
	/// Tangential tread depth / run (world meters).
	pub tread_depth: f32,
	/// Target rise per tread (~0.18 m per README) for uniform runs.
	pub tread_height: f32,
	/// Number of full turns over the run (1.0 = one revolution).
	pub turns: f32,
	/// Local tread-top \(Y\) values (ascending). Empty → uniform rise from
	/// [`Self::height`] / [`Self::tread_height`].
	pub tread_tops: Vec<f32>,
}

impl SpiralStair {
	pub const DEFAULT_TREAD_HEIGHT: f32 = 0.18;

	pub fn tread_count(&self) -> u32 {
		if !self.tread_tops.is_empty() {
			return self.tread_tops.len() as u32;
		}
		let h = self.tread_height.max(1e-4);
		(self.height / h).ceil().max(1.0) as u32
	}

	/// Exact rise per tread so a uniform run lands on `height`.
	pub fn rise_per_tread(&self) -> f32 {
		self.height / self.tread_count() as f32
	}

	/// Resolved ascending local tread-top \(Y\)s.
	pub fn effective_tread_tops(&self) -> Vec<f32> {
		if !self.tread_tops.is_empty() {
			return self.tread_tops.clone();
		}
		let n = self.tread_count();
		let rise = self.rise_per_tread();
		(1..=n).map(|i| i as f32 * rise).collect()
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
			tread_tops: Vec::new(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct StraightStair;

fn normalize_tops(mut tops: Vec<f32>) -> Vec<f32> {
	tops.retain(|y| y.is_finite());
	tops.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	tops.dedup_by(|a, b| (*a - *b).abs() < 1e-5);
	tops
}
