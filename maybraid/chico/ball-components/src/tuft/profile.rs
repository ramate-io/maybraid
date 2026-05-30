//! Shared width profile: belly bulge mid-strand, diamond tip taper.

/// Half-width along a strand at normalized height `t` ∈ [0, 1] (anchor → tip).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BellyTipProfile {
	pub base_half_width: f32,
	pub belly_half_width: f32,
}

impl BellyTipProfile {
	/// sin(πt) belly bulge, × (1−t) tip taper.
	pub fn half_width_at(&self, t: f32) -> f32 {
		let belly = (std::f32::consts::PI * t).sin().max(0.0);
		let width = self.base_half_width + (self.belly_half_width - self.base_half_width) * belly;
		width * (1.0 - t).max(0.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn belly_tip_profile_widens_then_tapers() -> Result<()> {
		let profile = BellyTipProfile {
			base_half_width: 0.02,
			belly_half_width: 0.06,
		};
		assert!(profile.half_width_at(0.5) > profile.half_width_at(0.0));
		assert!(profile.half_width_at(1.0) < 1e-5);
		Ok(())
	}
}
