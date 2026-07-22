//! Post-carve terrain height noise within a watershed footprint.
//!
//! Applied **after** apron + wet-core carves so hummocks / islands can rise
//! into an already-filled basin. Three intents:
//! - [`WatershedBackfillKind::Basin`] — anywhere inside the wet core
//! - [`WatershedBackfillKind::Rim`] — near / overlapping the rim (later)
//! - [`WatershedBackfillKind::Cell`] — anywhere in the leaf cell (later)
//!
//! Amplitude is **depth-incentive**: callers supply a freeboard (depth below
//! \(W\)) and a [`BasinBackfillParams::depth_frac`]. Consumers such as bog
//! stamps decide how aggressively to fill (peaking policy → `depth_frac`).

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
	/// When true, height noise only raises (`+|sample|`) — mound fill, no digs.
	pub add_only: bool,
}

impl WatershedBackfill {
	/// Basin backfill over `region` (typically a depression wet core).
	pub fn basin(region: Region2D, noise: RegionNoise, fade: f32) -> Self {
		Self {
			kind: WatershedBackfillKind::Basin,
			region,
			noise,
			fade: fade.max(0.25),
			add_only: false,
		}
	}

	/// Rim-band backfill (caller supplies the rim/annulus region).
	pub fn rim(region: Region2D, noise: RegionNoise, fade: f32) -> Self {
		Self {
			kind: WatershedBackfillKind::Rim,
			region,
			noise,
			fade: fade.max(0.25),
			add_only: false,
		}
	}

	/// Cell-wide backfill (caller supplies the cell region).
	pub fn cell(region: Region2D, noise: RegionNoise, fade: f32) -> Self {
		Self {
			kind: WatershedBackfillKind::Cell,
			region,
			noise,
			fade: fade.max(0.25),
			add_only: false,
		}
	}

	pub fn add_only(mut self) -> Self {
		self.add_only = true;
		self
	}

	/// Compile to an additive-in-region jersey op: `h' = h + (1−w)·noise`.
	///
	/// Uses affine with `inner_scale = 1`, `inner_offset = 0`, and optional
	/// raise-only height noise.
	pub fn into_modulation(self) -> JerseyModulation {
		let affine = RegionAffineModulation::new(self.region, 1.0, 0.0, 0.0, self.fade);
		JerseyModulation::Affine(if self.add_only {
			affine.with_height_noise_add_only(self.noise)
		} else {
			affine.with_height_noise(self.noise)
		})
	}
}

/// Depth-incentive authoring for a basin backfill noise draw.
///
/// World amplitude is `freeboard * depth_frac` via [`Self::amp_for_freeboard`].
/// Stamp facades (bog, later rim/cell recipes) choose `depth_frac` from their
/// own fill / peaking policy.
#[derive(Debug, Clone, Copy)]
pub struct BasinBackfillParams {
	/// Noise amplitude as a multiple of bowl freeboard (`1` ≈ full-scale raise
	/// equals depth below \(W\)).
	pub depth_frac: f32,
	pub freq: f32,
	pub fade: f32,
	/// FBM octave count (≥1). Extra octaves densify mound packing.
	pub octaves: u8,
	/// Raise-only mounds (no bipolar digs into the carved bed).
	pub add_only: bool,
}

impl Default for BasinBackfillParams {
	fn default() -> Self {
		Self {
			depth_frac: 1.0,
			freq: 0.04,
			fade: 2.0,
			octaves: 1,
			add_only: false,
		}
	}
}

impl BasinBackfillParams {
	/// `freeboard * depth_frac` (both non-negative).
	pub fn amp_for_freeboard(&self, freeboard: f32) -> f32 {
		freeboard.max(0.0) * self.depth_frac.max(0.0)
	}

	/// Sample over `region` with amplitude resolved from `freeboard`.
	pub fn sample_over_freeboard(
		&self,
		freeboard: f32,
		seed: u32,
		salt_offset: u32,
		region: Region2D,
	) -> WatershedBackfill {
		use procedural_common::{NoiseParams, NoiseType};

		let noise = RegionNoise::from_params(NoiseParams {
			seed: seed.wrapping_add(salt_offset) as i32,
			frequency: self.freq.max(1.0e-4).clamp(1.0e-4, 0.14),
			amplitude: self.amp_for_freeboard(freeboard),
			octaves: self.octaves.max(1) as u32,
			noise_type: NoiseType::Perlin,
		});
		let mut backfill = WatershedBackfill::basin(region, noise, self.fade);
		if self.add_only {
			backfill = backfill.add_only();
		}
		backfill
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

	#[test]
	fn amp_scales_with_freeboard_and_depth_frac() -> anyhow::Result<()> {
		let p = BasinBackfillParams {
			depth_frac: 1.25,
			..BasinBackfillParams::default()
		};
		assert!((p.amp_for_freeboard(8.0) - 10.0).abs() < 1e-4);
		assert!((p.amp_for_freeboard(4.0) - 5.0).abs() < 1e-4);
		Ok(())
	}
}
