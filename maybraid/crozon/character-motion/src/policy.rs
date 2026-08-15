//! Per-band motion stamps. Bake once in `scene_with_level`; switching bands shows
//! a different child — it does not rebuild the scene to flip a bool.

use lod::LodSceneLevel;

use crate::markers::{AnimateBones, AnimateEffects, ApplyTerrainPitch};

/// Which motion markers a level child should carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MotionPolicy {
	pub bones: bool,
	pub effects: bool,
	pub pitch: bool,
}

impl MotionPolicy {
	pub const HIGH: Self = Self { bones: true, effects: true, pitch: true };
	pub const LOW: Self = Self { bones: false, effects: true, pitch: true };
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

/// Band policy for character motion.
///
/// | Level | bones | effects | pitch |
/// |---|---|---|---|
/// | High, Medium | yes | yes | yes |
/// | Low | no | yes | yes |
/// | UltraLow / distance / resolution | no | no | no |
pub fn motion_policy(level: LodSceneLevel) -> MotionPolicy {
	match level {
		LodSceneLevel::High | LodSceneLevel::Medium => MotionPolicy::HIGH,
		LodSceneLevel::Low => MotionPolicy::LOW,
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
			MotionPolicy::NONE
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn high_has_all_markers() {
		let p = motion_policy(LodSceneLevel::High);
		assert!(p.bones && p.effects && p.pitch);
	}

	#[test]
	fn low_keeps_effects_and_pitch() {
		let p = motion_policy(LodSceneLevel::Low);
		assert!(!p.bones && p.effects && p.pitch);
	}

	#[test]
	fn ultra_low_is_silent() {
		let p = motion_policy(LodSceneLevel::UltraLow);
		assert_eq!(p, MotionPolicy::NONE);
	}
}
