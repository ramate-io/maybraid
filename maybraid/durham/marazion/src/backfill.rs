//! Post-carve terrain height noise within a watershed footprint.
//!
//! Applied **after** apron + wet-core carves so hummocks / islands can rise
//! into an already-filled basin. Three intents:
//! - [`WatershedBackfillKind::Basin`] — anywhere inside the wet core
//! - [`WatershedBackfillKind::Rim`] — near / overlapping the rim (later)
//! - [`WatershedBackfillKind::Cell`] — anywhere in the leaf cell (later)

use jersey_terrain_stamps::{JerseyModulation, Region2D, RegionAffineModulation, RegionNoise};

/// Which footprint a backfill samples within.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatershedBackfillKind {
	/// Inside the wet-core / basin region.
	Basin,
	/// Near and overlapping the rim shelf (region supplied by caller; later).
	Rim,
	/// Full leaf / cell footprint (region supplied by caller; later).
	Cell,
}

/// Softmask height-noise stamped after water carve.
#[derive(Debug, Clone)]
pub struct WatershedBackfill {
	pub kind: WatershedBackfillKind,
	pub region: Region2D,
	pub noise: RegionNoise,
	/// Softmask fade past the region SDF zero (world units).
	pub fade: f32,
}

impl WatershedBackfill {
	/// Basin backfill over `region` (typically a depression wet core).
	pub fn basin(region: Region2D, noise: RegionNoise, fade: f32) -> Self {
		Self {
			kind: WatershedBackfillKind::Basin,
			region,
			noise,
			fade: fade.max(0.25),
		}
	}

	/// Rim-band backfill (caller supplies the rim/annulus region).
	pub fn rim(region: Region2D, noise: RegionNoise, fade: f32) -> Self {
		Self {
			kind: WatershedBackfillKind::Rim,
			region,
			noise,
			fade: fade.max(0.25),
		}
	}

	/// Cell-wide backfill (caller supplies the cell region).
	pub fn cell(region: Region2D, noise: RegionNoise, fade: f32) -> Self {
		Self {
			kind: WatershedBackfillKind::Cell,
			region,
			noise,
			fade: fade.max(0.25),
		}
	}

	/// Compile to an additive-in-region jersey op: `h' = h + (1−w)·noise`.
	///
	/// Uses affine with `inner_scale = 1`, `inner_offset = 0`, bipolar height noise.
	pub fn into_modulation(self) -> JerseyModulation {
		JerseyModulation::Affine(
			RegionAffineModulation::new(self.region, 1.0, 0.0, 0.0, self.fade)
				.with_height_noise(self.noise),
		)
	}
}

/// Authoring knobs for a basin backfill noise draw.
#[derive(Debug, Clone, Copy)]
pub struct BasinBackfillParams {
	pub amp: f32,
	pub freq: f32,
	pub fade: f32,
}

impl Default for BasinBackfillParams {
	fn default() -> Self {
		Self {
			amp: 6.0,
			freq: 0.04,
			fade: 2.0,
		}
	}
}

impl BasinBackfillParams {
	/// Sample noise and wrap a basin backfill on `region`.
	pub fn sample(&self, seed: u32, salt_offset: u32, region: Region2D) -> WatershedBackfill {
		let noise = RegionNoise::from_seed(
			seed.wrapping_add(salt_offset),
			self.freq.max(1.0e-4).clamp(1.0e-4, 0.14),
			self.amp.max(0.0),
		);
		WatershedBackfill::basin(region, noise, self.fade)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec2;
	use jersey_terrain_stamps::CircleRegion;

	#[test]
	fn basin_backfill_moves_inside_identity_outside() -> anyhow::Result<()> {
		let region = Region2D::Circle(CircleRegion {
			center: Vec2::ZERO,
			radius: 20.0,
		});
		let noise = RegionNoise::from_seed(7, 0.05, 5.0);
		let m = WatershedBackfill::basin(region, noise, 2.0).into_modulation();
		let base = 40.0;
		// Perlin can be ~0 on lattice points; probe a few interior samples.
		let mut max_delta = 0.0_f32;
		for &(x, z) in &[(0.0, 0.0), (3.0, 5.0), (-7.0, 2.0), (4.0, -6.0)] {
			max_delta = max_delta.max((m.modify_elevation(base, x, z) - base).abs());
		}
		let outside = m.modify_elevation(base, 80.0, 0.0);
		assert!(
			max_delta > 0.1,
			"inside should receive height noise (max |Δ|={max_delta})"
		);
		assert!(
			(outside - base).abs() < 1e-3,
			"outside should stay identity: {outside} vs {base}"
		);
		Ok(())
	}
}
