//! Blend continuous flapping into a quiet soar glide.

use super::{Flapping, Soaring};

pub const GLIDE_FROM_FLAP_BLEND_SECS: f32 = 0.4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlideFromFlap {
	pub flap: Flapping,
	pub soar: Soaring,
	pub flap_elapsed_start: f32,
	pub blend_secs: f32,
}

impl Default for GlideFromFlap {
	fn default() -> Self {
		Self {
			flap: Flapping::default(),
			soar: Soaring::default(),
			flap_elapsed_start: 0.0,
			blend_secs: GLIDE_FROM_FLAP_BLEND_SECS,
		}
	}
}

impl GlideFromFlap {
	pub fn soar_glide_progress(&self) -> f32 {
		self.soar.burst_duration() + self.soar.pause.max(0.0) * 0.5
	}

	pub fn with_flap(mut self, flap: Flapping) -> Self {
		self.flap = flap;
		self
	}

	pub fn with_flap_elapsed_start(mut self, elapsed: f32) -> Self {
		self.flap_elapsed_start = elapsed.max(0.0);
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
	fn source_phase_is_preserved_and_target_is_quiet_glide() -> anyhow::Result<()> {
		let transition = GlideFromFlap::default().with_flap_elapsed_start(0.3);
		let sample = transition.soar_glide_progress();
		assert!(sample > transition.soar.burst_duration());
		assert!(sample < transition.soar.cycle_duration());
		assert!(transition.soar.flap_amount(sample).abs() < 1e-5);
		assert_eq!(transition.flap_elapsed_start, 0.3);
		assert_eq!(transition.with_flap_elapsed_start(-1.0).flap_elapsed_start, 0.0);
		Ok(())
	}
}
