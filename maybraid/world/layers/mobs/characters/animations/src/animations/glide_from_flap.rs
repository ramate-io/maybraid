//! Blend from continuous wing flapping into a held soar glide.
//!
//! `progress` is wall-clock seconds. The first [`GLIDE_FROM_FLAP_BLEND_SECS`] seconds
//! mix [`Flapping`] into [`Soaring`] sampled at a glide pause (zero flap phase), then
//! the clip continues as pure soar with elapsed time.

use super::{Flapping, Soaring};

/// Seconds spent mixing flapping into soar glide before the clip is pure soar.
pub const GLIDE_FROM_FLAP_BLEND_SECS: f32 = 0.4;

/// Continuous flapping blended into soaring glide at a quiet pause between bursts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlideFromFlap {
	pub flap: Flapping,
	pub soar: Soaring,
	/// Wall-clock blend window at clip start.
	pub blend_secs: f32,
}

impl Default for GlideFromFlap {
	fn default() -> Self {
		Self {
			flap: Flapping::default(),
			soar: Soaring::default(),
			blend_secs: GLIDE_FROM_FLAP_BLEND_SECS,
		}
	}
}

impl GlideFromFlap {
	/// Soaring progress at the quiet glide between bursts (flap_amount ≈ 0).
	pub fn soar_glide_progress(&self) -> f32 {
		self.soar.burst_duration() + self.soar.pause * 0.5
	}

	pub fn with_flap(mut self, flap: Flapping) -> Self {
		self.flap = flap;
		self
	}

	pub fn with_soar(mut self, soar: Soaring) -> Self {
		self.soar = soar;
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
		let exit = GlideFromFlap::default();
		let soar = exit.soar;
		let t = exit.soar_glide_progress();
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
