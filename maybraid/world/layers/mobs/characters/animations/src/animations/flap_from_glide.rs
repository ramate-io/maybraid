//! Blend from a held soar glide into continuous wing flapping.
//!
//! `progress` is wall-clock seconds. The first [`FLAP_FROM_GLIDE_BLEND_SECS`] seconds
//! mix [`Soaring`] sampled at a glide pause (zero flap phase) into [`Flapping`] with
//! separate clip progress via `apply_at`-style sampling.

use super::{Flapping, Soaring};

/// Seconds spent mixing soar glide into flapping before the clip is pure flap.
pub const FLAP_FROM_GLIDE_BLEND_SECS: f32 = 0.4;

/// Soaring glide sampled at mid-pause (flap_amount ≈ 0) blended into continuous flapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlapFromGlide {
	pub soar: Soaring,
	pub flap: Flapping,
	/// Wall-clock blend window at clip start.
	pub blend_secs: f32,
}

impl Default for FlapFromGlide {
	fn default() -> Self {
		Self {
			soar: Soaring::default(),
			flap: Flapping::default(),
			blend_secs: FLAP_FROM_GLIDE_BLEND_SECS,
		}
	}
}

impl FlapFromGlide {
	/// Soaring progress during the quiet glide between bursts (flap phase aligned with flapping at `0`).
	pub fn soar_glide_progress(&self) -> f32 {
		self.soar.burst_duration() + self.soar.pause * 0.5
	}

	pub fn with_soar(mut self, soar: Soaring) -> Self {
		self.soar = soar;
		self
	}

	pub fn with_flap(mut self, flap: Flapping) -> Self {
		self.flap = flap;
		self
	}

	pub fn with_blend_secs(mut self, blend_secs: f32) -> Self {
		self.blend_secs = blend_secs.max(0.0);
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn glide_progress_lands_in_pause() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let soar = entry.soar;
		let t = entry.soar_glide_progress();
		assert!(t > soar.burst_duration(), "glide sample should be past the burst");
		assert!(t < soar.cycle_duration(), "glide sample should stay inside the cycle");
		assert!(soar.flap_amount(t).abs() < 1e-4, "glide pause keeps wings neutral");
		Ok(())
	}

	#[test]
	fn flapping_starts_at_zero_flap_phase() -> anyhow::Result<()> {
		let flap = Flapping::default();
		assert!(flap.flap_amount(0.0).abs() < 1e-5);
		Ok(())
	}
}
