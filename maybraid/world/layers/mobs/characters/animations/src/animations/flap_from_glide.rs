//! Blend a quiet soar-glide pose into continuous flapping.

use super::{Flapping, Soaring};

pub const FLAP_FROM_GLIDE_BLEND_SECS: f32 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlapFromGlide {
	pub soar: Soaring,
	pub flap: Flapping,
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
	pub fn soar_glide_progress(&self) -> f32 {
		self.soar.burst_duration() + self.soar.pause.max(0.0) * 0.5
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
	fn source_sample_is_in_glide_pause_and_blend_is_bounded() -> anyhow::Result<()> {
		let transition = FlapFromGlide::default();
		let sample = transition.soar_glide_progress();
		assert!(sample > transition.soar.burst_duration());
		assert!(sample < transition.soar.cycle_duration());
		assert!(transition.soar.flap_amount(sample).abs() < 1e-5);
		assert_eq!(transition.with_blend_secs(-1.0).blend_secs, 0.0);
		Ok(())
	}
}
