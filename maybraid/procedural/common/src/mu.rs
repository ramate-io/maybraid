//! Conservative **band margin** for SDF operations (meshing / bounds / chunk `mu`) derived from [`NoiseParams`].
//!
//! Surface displacement from noise is bounded in magnitude by roughly [`NoiseParams::amplitude`], but
//! fractal combinations can add high-frequency overshoot. [`sdf_band_margin`] adds a small numerical
//! floor plus an octave-aware slack term—more generous than `amplitude + ε` alone while staying
//! O(authoring parameters).

use crate::NoiseParams;

/// Small pad so rasterization / chunk queries do not clip the zero level set.
pub const NUMERIC_SURFACE_EPSILON: f32 = 1e-3;

/// Half-width style margin for bound expansion and chunk **mu** (query band around mesh content).
///
/// Includes [`NoiseParams::amplitude`] magnitude, optional extra slack when **`octaves > 1`** (FBm),
/// and [`NUMERIC_SURFACE_EPSILON`].
pub fn sdf_band_margin(params: &NoiseParams) -> f32 {
	let a = params.amplitude.abs();
	let fractal_slack = if params.octaves <= 1 {
		0.0
	} else {
		// Conservative crest overshoot vs single octave; grows sublinearly with extra octaves.
		a * 0.35 * ((params.octaves - 1) as f32).sqrt()
	};
	a + fractal_slack + NUMERIC_SURFACE_EPSILON
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::NoiseParams;
	use anyhow::Result;

	#[test]
	fn margin_at_least_amplitude_plus_eps() -> Result<()> {
		let p = NoiseParams { amplitude: 0.05, octaves: 1, ..Default::default() };
		let m = sdf_band_margin(&p);
		assert!(m >= p.amplitude.abs() + NUMERIC_SURFACE_EPSILON);
		Ok(())
	}

	#[test]
	fn multi_octave_exceeds_single() -> Result<()> {
		let base = NoiseParams { amplitude: 0.1, ..Default::default() };
		let one = NoiseParams { octaves: 1, ..base };
		let three = NoiseParams { octaves: 3, ..base };
		assert!(sdf_band_margin(&three) > sdf_band_margin(&one));
		Ok(())
	}
}
