//! Per-band motion stamps for **host** markers. Bake defaults at spawn;
//! [`crate::sync::sync_motion_markers`] keeps them aligned with the shown LOD band.
//!
//! [`motion_policy`] is the default linear ramp, not a registry of regimes.

use intelligence_lod::{IntelligenceBand, IntelligenceLod};
use lod::LodSceneLevel;

use crate::markers::{AnimateBones, AnimateEffects, ApplyTerrainPitch};

/// Which motion markers a host should carry for a level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionPolicy {
	pub bones: bool,
	pub effects: bool,
	pub pitch: bool,
}

impl MotionPolicy {
	pub const HIGH: Self = Self { bones: true, effects: true, pitch: true };
	pub const MEDIUM: Self = Self { bones: false, effects: true, pitch: true };
	pub const LOW: Self = Self { bones: false, effects: true, pitch: false };
	pub const NONE: Self = Self { bones: false, effects: false, pitch: false };

	pub fn animate_bones(self) -> Option<AnimateBones> {
		self.bones.then_some(AnimateBones)
	}

	pub fn animate_effects(self) -> Option<AnimateEffects> {
		self.effects.then_some(AnimateEffects)
	}

	pub fn apply_terrain_pitch(self) -> Option<ApplyTerrainPitch> {
		self.pitch.then_some(ApplyTerrainPitch)
	}
}

/// Default linear [`LodSceneLevel`] → [`MotionPolicy`] map.
///
/// One shared ramp for every character recipe: nearer bands keep more work,
/// farther bands drop it. This is **not** a pluggable regime — a species that
/// needs a different map should sync different host markers itself.
///
/// | Level | bones | effects | pitch |
/// |---|---|---|---|
/// | High | yes | yes | yes |
/// | Medium | no | yes | yes |
/// | Low | no | yes | no |
/// | UltraLow / distance / resolution | no | no | no |
pub fn motion_policy(level: LodSceneLevel) -> MotionPolicy {
	match level {
		LodSceneLevel::High => MotionPolicy::HIGH,
		LodSceneLevel::Medium => MotionPolicy::MEDIUM,
		LodSceneLevel::Low => MotionPolicy::LOW,
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
			MotionPolicy::NONE
		}
	}
}

/// Drop mailbox and pitch work on Mid / Far plants. Missing lod is Near
/// (local player).
///
/// Character `LodScene` stays High — this is not a scene cull. Effects go with
/// bones so [`crate::apply_anim_mailbox`] does not keep the expensive path.
pub fn clamp_intelligence(mut policy: MotionPolicy, lod: Option<&IntelligenceLod>) -> MotionPolicy {
	if IntelligenceLod::band_or_near(lod) != IntelligenceBand::Near {
		policy.bones = false;
		policy.effects = false;
		policy.pitch = false;
	}
	policy
}

#[cfg(test)]
mod tests {
	use super::*;
	use intelligence_lod::{IntelligenceBand, IntelligenceLod};

	#[test]
	fn high_has_all_markers() {
		let p = motion_policy(LodSceneLevel::High);
		assert!(p.bones && p.effects && p.pitch);
	}

	#[test]
	fn medium_keeps_effects_and_pitch() {
		let p = motion_policy(LodSceneLevel::Medium);
		assert_eq!(p, MotionPolicy::MEDIUM);
		assert!(!p.bones && p.effects && p.pitch);
	}

	#[test]
	fn low_keeps_effects_only() {
		let p = motion_policy(LodSceneLevel::Low);
		assert_eq!(p, MotionPolicy::LOW);
		assert!(!p.bones && p.effects && !p.pitch);
	}

	#[test]
	fn ultra_low_is_silent() {
		let p = motion_policy(LodSceneLevel::UltraLow);
		assert_eq!(p, MotionPolicy::NONE);
	}

	#[test]
	fn missing_and_near_keep_high_mailbox() {
		let high = motion_policy(LodSceneLevel::High);
		assert_eq!(clamp_intelligence(high, None), MotionPolicy::HIGH);
		assert_eq!(clamp_intelligence(high, Some(&IntelligenceLod::missing())), MotionPolicy::HIGH);
	}

	#[test]
	fn mid_and_far_drop_mailbox_and_pitch() {
		let high = motion_policy(LodSceneLevel::High);
		let mid = IntelligenceLod { band: IntelligenceBand::Mid, skips: 0 };
		let far = IntelligenceLod { band: IntelligenceBand::Far, skips: 0 };
		let mid_p = clamp_intelligence(high, Some(&mid));
		let far_p = clamp_intelligence(high, Some(&far));
		assert!(!mid_p.bones && !mid_p.effects && !mid_p.pitch);
		assert!(!far_p.bones && !far_p.effects && !far_p.pitch);
	}
}
