//! Despawn policy for inactive [`crate::lod_scene_host::LodLevelRoot`]s.

use crate::lod_level::LodSceneLevel;

/// One cull target: a concrete level or an open-ended custom category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodSceneCull {
	/// Despawn a root keyed by this exact [`LodSceneLevel`].
	Level(LodSceneLevel),
	/// Despawn every [`LodSceneLevel::Distance`] root.
	AllDistance,
	/// Despawn every [`LodSceneLevel::Resolution`] root.
	AllResolution,
}

impl LodSceneCull {
	/// Whether this cull entry matches `level`.
	pub fn matches(self, level: LodSceneLevel) -> bool {
		match self {
			Self::Level(wanted) => wanted == level,
			Self::AllDistance => matches!(level, LodSceneLevel::Distance(_)),
			Self::AllResolution => matches!(level, LodSceneLevel::Resolution(_)),
		}
	}
}

/// Which inactive LOD level roots a [`crate::gen::LodScene`] is willing to despawn.
///
/// Default [`Self::None`] keeps hidden roots warm. Prefer explicit tight
/// [`Self::AllOf`] lists when memory matters; do not treat “not current” as
/// an automatic cull.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum LodSceneCulls {
	/// Despawn nothing (default).
	#[default]
	None,
	/// Despawn every matching inactive root.
	AllOf(Vec<LodSceneCull>),
}

impl LodSceneCulls {
	/// Whether `level` is listed for despawn.
	///
	/// Callers must still skip the host's current/desired level.
	pub fn should_cull(&self, level: LodSceneLevel) -> bool {
		match self {
			Self::None => false,
			Self::AllOf(entries) => entries.iter().any(|c| c.matches(level)),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::lod_level::QuantizedDistance;

	#[test]
	fn none_culls_nothing() {
		assert!(!LodSceneCulls::None.should_cull(LodSceneLevel::High));
	}

	#[test]
	fn all_of_matches_level_and_customs() {
		let culls = LodSceneCulls::AllOf(vec![
			LodSceneCull::Level(LodSceneLevel::High),
			LodSceneCull::AllDistance,
			LodSceneCull::AllResolution,
		]);
		assert!(culls.should_cull(LodSceneLevel::High));
		assert!(!culls.should_cull(LodSceneLevel::Medium));
		assert!(culls.should_cull(LodSceneLevel::Distance(QuantizedDistance(3))));
		assert!(culls.should_cull(LodSceneLevel::Resolution(16)));
	}
}
