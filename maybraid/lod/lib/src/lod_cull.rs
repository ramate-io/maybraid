//! Despawn policy for inactive [`crate::lod_scene_host::LodLevelRoot`]s.
//!
//! Common helpers:
//! - [`cull_non_adjacent_bands`] — least aggressive named-band GC
//! - [`cull_bands_with_adjacent_depth`] — also drop the nearer adjacent band once
//!   you are far enough into the current band

use crate::lod_level::LodSceneLevel;

/// Named presentation bands ordered **near → far** (detail → silhouette).
pub const NAMED_BANDS_NEAR_TO_FAR: [LodSceneLevel; 4] = [
	LodSceneLevel::High,
	LodSceneLevel::Medium,
	LodSceneLevel::Low,
	LodSceneLevel::UltraLow,
];

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

	/// Append cull entries (dedupes exact [`LodSceneCull`] values).
	pub fn and(self, extra: impl IntoIterator<Item = LodSceneCull>) -> Self {
		let mut entries = match self {
			Self::None => Vec::new(),
			Self::AllOf(v) => v,
		};
		for c in extra {
			if !entries.contains(&c) {
				entries.push(c);
			}
		}
		if entries.is_empty() {
			Self::None
		} else {
			Self::AllOf(entries)
		}
	}

	/// Append a concrete level cull.
	pub fn and_level(self, level: LodSceneLevel) -> Self {
		self.and([LodSceneCull::Level(level)])
	}

	/// Append [`LodSceneCull::AllDistance`] and [`LodSceneCull::AllResolution`].
	pub fn with_customs(self) -> Self {
		self.and([LodSceneCull::AllDistance, LodSceneCull::AllResolution])
	}
}

/// Index on [`NAMED_BANDS_NEAR_TO_FAR`], if `level` is a named band.
pub fn named_band_index(level: LodSceneLevel) -> Option<usize> {
	match level {
		LodSceneLevel::High => Some(0),
		LodSceneLevel::Medium => Some(1),
		LodSceneLevel::Low => Some(2),
		LodSceneLevel::UltraLow => Some(3),
		LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => None,
	}
}

/// Least aggressive named-band GC: cull bands with index distance &gt; 1.
///
/// Examples (current → culls):
/// - High → Low, UltraLow (Medium stays warm)
/// - Medium → UltraLow (High and Low stay warm)
/// - Low → High (Medium and UltraLow stay warm)
/// - UltraLow → High, Medium (Low stays warm)
///
/// Customs (`Distance` / `Resolution`) are not included; chain
/// [`LodSceneCulls::with_customs`] if needed. Unknown current → [`LodSceneCulls::None`].
pub fn cull_non_adjacent_bands(current: LodSceneLevel) -> LodSceneCulls {
	let Some(i) = named_band_index(current) else {
		return LodSceneCulls::None;
	};
	let entries: Vec<LodSceneCull> = NAMED_BANDS_NEAR_TO_FAR
		.into_iter()
		.enumerate()
		.filter(|(j, _)| j.abs_diff(i) > 1)
		.map(|(_, level)| LodSceneCull::Level(level))
		.collect();
	if entries.is_empty() {
		LodSceneCulls::None
	} else {
		LodSceneCulls::AllOf(entries)
	}
}

/// [`cull_non_adjacent_bands`], and also cull the **nearer** (more detailed) adjacent
/// band once `progress_into_band` ≥ `depth`.
///
/// `progress_into_band` is 0 at the near edge of the current band (just left the
/// finer neighbor) and 1 at the far edge (about to enter the coarser neighbor).
///
/// Example: current Medium, `depth = 0.5`, `progress = 0.6` → UltraLow (non-adjacent)
/// plus High (nearer adjacent). Early in Medium (`progress = 0.2`) High stays warm.
///
/// `depth ≤ 0` culls the nearer adjacent as soon as the band is entered.
/// `depth > 1` never adds the adjacent (same as non-adjacent only).
pub fn cull_bands_with_adjacent_depth(
	current: LodSceneLevel,
	progress_into_band: f32,
	depth: f32,
) -> LodSceneCulls {
	let base = cull_non_adjacent_bands(current);
	let Some(i) = named_band_index(current) else {
		return base;
	};
	if i == 0 || progress_into_band < depth {
		return base;
	}
	base.and_level(NAMED_BANDS_NEAR_TO_FAR[i - 1])
}

/// Named band and 0..=1 progress through it from a distance/extent `factor`.
///
/// Thresholds are the far edges of High / Medium / Low (same convention as
/// domain probes: `factor ≤ high` → High, etc.).
pub fn named_band_progress(
	factor: f32,
	high: f32,
	medium: f32,
	low: f32,
) -> (LodSceneLevel, f32) {
	let factor = factor.max(0.0);
	if factor <= high {
		let progress = if high > 1e-4 { (factor / high).clamp(0.0, 1.0) } else { 1.0 };
		(LodSceneLevel::High, progress)
	} else if factor <= medium {
		let span = (medium - high).max(1e-4);
		(LodSceneLevel::Medium, ((factor - high) / span).clamp(0.0, 1.0))
	} else if factor <= low {
		let span = (low - medium).max(1e-4);
		(LodSceneLevel::Low, ((factor - medium) / span).clamp(0.0, 1.0))
	} else {
		// No authored far edge for UltraLow — treat as fully into the band.
		(LodSceneLevel::UltraLow, 1.0)
	}
}

/// Named-band culls from a distance/extent factor.
///
/// - `adjacent_depth: None` → [`cull_non_adjacent_bands`]
/// - `adjacent_depth: Some(d)` → [`cull_bands_with_adjacent_depth`] with progress
///   from [`named_band_progress`]
pub fn cull_named_from_factor(
	factor: f32,
	high: f32,
	medium: f32,
	low: f32,
	adjacent_depth: Option<f32>,
) -> LodSceneCulls {
	let (level, progress) = named_band_progress(factor, high, medium, low);
	match adjacent_depth {
		None => cull_non_adjacent_bands(level),
		Some(depth) => cull_bands_with_adjacent_depth(level, progress, depth),
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

	#[test]
	fn non_adjacent_examples() {
		let high = cull_non_adjacent_bands(LodSceneLevel::High);
		assert!(high.should_cull(LodSceneLevel::Low));
		assert!(high.should_cull(LodSceneLevel::UltraLow));
		assert!(!high.should_cull(LodSceneLevel::Medium));
		assert!(!high.should_cull(LodSceneLevel::High));

		let medium = cull_non_adjacent_bands(LodSceneLevel::Medium);
		assert!(medium.should_cull(LodSceneLevel::UltraLow));
		assert!(!medium.should_cull(LodSceneLevel::High));
		assert!(!medium.should_cull(LodSceneLevel::Low));

		let low = cull_non_adjacent_bands(LodSceneLevel::Low);
		assert!(low.should_cull(LodSceneLevel::High));
		assert!(!low.should_cull(LodSceneLevel::Medium));
		assert!(!low.should_cull(LodSceneLevel::UltraLow));

		let ultra = cull_non_adjacent_bands(LodSceneLevel::UltraLow);
		assert!(ultra.should_cull(LodSceneLevel::High));
		assert!(ultra.should_cull(LodSceneLevel::Medium));
		assert!(!ultra.should_cull(LodSceneLevel::Low));
	}

	#[test]
	fn adjacent_depth_culls_high_well_into_medium() {
		let early = cull_bands_with_adjacent_depth(LodSceneLevel::Medium, 0.2, 0.5);
		assert!(!early.should_cull(LodSceneLevel::High));
		assert!(early.should_cull(LodSceneLevel::UltraLow));

		let deep = cull_bands_with_adjacent_depth(LodSceneLevel::Medium, 0.6, 0.5);
		assert!(deep.should_cull(LodSceneLevel::High));
		assert!(deep.should_cull(LodSceneLevel::UltraLow));
	}

	#[test]
	fn adjacent_depth_zero_culls_nearer_immediately() {
		let mid = cull_bands_with_adjacent_depth(LodSceneLevel::Medium, 0.0, 0.0);
		assert!(mid.should_cull(LodSceneLevel::High));
	}

	#[test]
	fn named_band_progress_spans() {
		let (level, p) = named_band_progress(2.5, 5.0, 20.0, 500.0);
		assert_eq!(level, LodSceneLevel::High);
		assert!((p - 0.5).abs() < 1e-4);

		let (level, p) = named_band_progress(12.5, 5.0, 20.0, 500.0);
		assert_eq!(level, LodSceneLevel::Medium);
		assert!((p - 0.5).abs() < 1e-4);

		let (level, p) = named_band_progress(1000.0, 5.0, 20.0, 500.0);
		assert_eq!(level, LodSceneLevel::UltraLow);
		assert_eq!(p, 1.0);
	}

	#[test]
	fn with_customs_appends() {
		let culls = cull_non_adjacent_bands(LodSceneLevel::Low).with_customs();
		assert!(culls.should_cull(LodSceneLevel::High));
		assert!(culls.should_cull(LodSceneLevel::Distance(QuantizedDistance(1))));
		assert!(culls.should_cull(LodSceneLevel::Resolution(8)));
	}
}
