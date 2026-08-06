//! Frond collection geometry — many placed frond kits under one foliage LOD parent.
//!
//! Analogous to polyline / tessellated partitions in Richmond building-components:
//! the continuous form is a [`FoliageGeometry::FrondCollection`](crate::FoliageGeometry)
//! on a single [`FoliageNode`](crate::FoliageNode); leaf kits are expanded per LOD band.
//!
//! Connectivity is authored as [`FrondRun`]s (ordered base→tip chains). LOD merge drops
//! or collapses whole runs so kinked blades stay connected.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;

use crate::placed::Placement;
use crate::procedural::FROND_KIT_HALF_X;

/// Collection LOD bands: `distance / collection_max_extent`.
///
/// Defined here (used by [`crate::FoliageLodProbe::for_frond_collection`]):
/// - High when factor ≤ [`FROND_COLLECTION_HIGH_FACTOR`]
/// - Medium when factor ≤ [`FROND_COLLECTION_MEDIUM_FACTOR`] (keep ≈ half **runs**, full chains)
/// - Low when factor ≤ [`FROND_COLLECTION_LOW_FACTOR`] (keep ≈ quarter runs, collapse each to a chord)
/// - UltraLow beyond that (one marker chord; real [`LodSceneLevel::UltraLow`] tier)
pub const FROND_COLLECTION_HIGH_FACTOR: f32 = 5.0;
/// See [`FROND_COLLECTION_HIGH_FACTOR`].
pub const FROND_COLLECTION_MEDIUM_FACTOR: f32 = 36.0;
/// See [`FROND_COLLECTION_HIGH_FACTOR`].
pub const FROND_COLLECTION_LOW_FACTOR: f32 = 90.0;

/// Which straight-frond kit a collection member instances.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrondKit {
	/// Point tip (`straight_frond_001_*`).
	StraightFrond,
	/// Square-ended segment (`straight_frond_segment_001_*`).
	StraightFrondSegment,
}

impl Default for FrondKit {
	fn default() -> Self {
		Self::StraightFrondSegment
	}
}

/// One placed frond leaf inside a [`FrondRun`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrondMember {
	pub kit: FrondKit,
	pub placement: Placement,
}

impl FrondMember {
	pub fn segment(placement: Placement) -> Self {
		Self { kit: FrondKit::StraightFrondSegment, placement }
	}

	pub fn tip(placement: Placement) -> Self {
		Self { kit: FrondKit::StraightFrond, placement }
	}

	fn tip_point(self) -> Vec3 {
		self.placement.translation
			+ self.placement.rotation() * Vec3::new(0.0, self.placement.scale.y.abs(), 0.0)
	}

	fn world_width(self) -> f32 {
		(self.placement.scale.x.abs() * 2.0 * FROND_KIT_HALF_X).max(1e-4)
	}

	fn with_width_scale(mut self, factor: f32) -> Self {
		self.placement.scale.x *= factor;
		self.placement.scale.z *= factor;
		self
	}
}

/// One connected blade (ordered base → tip segment chain).
#[derive(Debug, Clone, PartialEq)]
pub struct FrondRun {
	pub segments: Vec<FrondMember>,
}

impl FrondRun {
	pub fn new(segments: impl IntoIterator<Item = FrondMember>) -> Self {
		Self { segments: segments.into_iter().collect() }
	}

	pub fn from_placements(placements: impl IntoIterator<Item = Placement>) -> Self {
		Self::new(placements.into_iter().map(FrondMember::segment))
	}

	pub fn is_empty(&self) -> bool {
		self.segments.is_empty()
	}

	/// Sum of segment lengths along the chain.
	pub fn chain_length(&self) -> f32 {
		self.segments.iter().map(|s| s.placement.scale.y.abs()).sum()
	}

	fn with_width_scale(mut self, factor: f32) -> Self {
		for seg in &mut self.segments {
			*seg = seg.with_width_scale(factor);
		}
		self
	}

	/// Collapse the chain to a single base→tip chord (keeps connectivity as one leaf).
	///
	/// `width_factor` scales the resulting blade width (absorb dropped runs / segments).
	pub fn collapse_to_chord(&self, width_factor: f32) -> Option<FrondMember> {
		let first = *self.segments.first()?;
		let last = *self.segments.last()?;
		let start = first.placement.translation;
		let tip = last.tip_point();
		let ray = tip - start;
		let length = ray.length();
		if length < 1e-6 {
			return None;
		}
		let width = first.world_width() * width_factor * (self.segments.len() as f32).max(1.0);
		Placement::frond_segment(start, ray, length, width).map(FrondMember::segment)
	}
}

/// Continuous frond-collection form: many leaf kits, one LOD parent.
///
/// Authored as [`FrondRun`]s. Merge drops whole runs (and may collapse a kept run to a
/// chord) so kinked blades never lose mid-chain connectivity.
#[derive(Debug, Clone, PartialEq)]
pub struct FrondCollection {
	pub runs: Vec<FrondRun>,
}

impl FrondCollection {
	pub fn new(runs: impl IntoIterator<Item = FrondRun>) -> Self {
		Self { runs: runs.into_iter().collect() }
	}

	/// One single-segment run per placement (no authored connectivity).
	pub fn singleton_runs(placements: impl IntoIterator<Item = Placement>) -> Self {
		Self::new(placements.into_iter().map(|p| FrondRun::from_placements([p])))
	}

	pub fn is_empty(&self) -> bool {
		self.runs.iter().all(FrondRun::is_empty)
	}

	/// Axis-aligned bounds of all segment bases/tips expanded by blade half-width.
	pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		let mut any = false;
		for run in &self.runs {
			for member in &run.segments {
				any = true;
				let base = member.placement.translation;
				let tip = member.tip_point();
				let half_w = (member.placement.scale.x.abs() * FROND_KIT_HALF_X).max(1e-4);
				for p in [base, tip] {
					min = min.min(p - Vec3::splat(half_w));
					max = max.max(p + Vec3::splat(half_w));
				}
			}
		}
		any.then_some((min, max))
	}

	/// Center and max half-extent of the collection AABB (LOD distance unit).
	pub fn center_and_extent(&self) -> (Vec3, f32) {
		match self.aabb() {
			Some((min, max)) => {
				let center = (min + max) * 0.5;
				let extent = ((max - min) * 0.5).max_element().max(1e-4);
				(center, extent)
			}
			None => (Vec3::ZERO, 1e-4),
		}
	}

	/// Runs presented at `level` after merge thinning.
	pub fn runs_for_level(&self, level: LodSceneLevel) -> Vec<FrondRun> {
		let n = self.runs.len();
		if n == 0 {
			return Vec::new();
		}
		let (target, collapse) = match level {
			LodSceneLevel::High => (n, false),
			LodSceneLevel::Medium => (n.div_ceil(2).max(1), false),
			LodSceneLevel::Low => (n.div_ceil(4).max(1), true),
			LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
				(1, true)
			}
		};
		merge_runs(&self.runs, target, collapse)
	}

	/// Flattened leaf members at `level` (for scene emission).
	pub fn members_for_level(&self, level: LodSceneLevel) -> Vec<FrondMember> {
		self.runs_for_level(level).into_iter().flat_map(|run| run.segments).collect()
	}
}

/// Merge `runs` down to `target` survivors.
///
/// Each survivor absorbs a contiguous group of runs: width scales by group size.
/// When `collapse`, the survivor becomes a single base→tip chord.
fn merge_runs(runs: &[FrondRun], target: usize, collapse: bool) -> Vec<FrondRun> {
	let n = runs.len();
	if n == 0 {
		return Vec::new();
	}
	let target = target.clamp(1, n);
	if target == n && !collapse {
		return runs.to_vec();
	}
	let mut out = Vec::with_capacity(target);
	for k in 0..target {
		let start = (k * n) / target;
		let end = ((k + 1) * n) / target;
		let group = &runs[start..end];
		let factor = (group.len() as f32).max(1.0);
		let mut best = &group[0];
		for candidate in group.iter().skip(1) {
			if candidate.chain_length() > best.chain_length() {
				best = candidate;
			}
		}
		if collapse {
			if let Some(chord) = best.collapse_to_chord(factor) {
				out.push(FrondRun::new([chord]));
			}
		} else {
			out.push(best.clone().with_width_scale(factor));
		}
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn segment(start: Vec3, dir: Vec3, length: f32, width: f32) -> FrondMember {
		FrondMember::segment(
			Placement::frond_segment(start, dir, length, width).expect("placement"),
		)
	}

	fn run_of(segments: impl IntoIterator<Item = FrondMember>) -> FrondRun {
		FrondRun::new(segments)
	}

	#[test]
	fn merge_drops_runs_not_mid_chain_segments() -> Result<()> {
		// Four blades, each a 2-segment kinked chain.
		let collection = FrondCollection::new((0..4).map(|i| {
			let x = i as f32 * 0.2;
			run_of([
				segment(Vec3::new(x, 0.0, 0.0), Vec3::Y, 0.5, 0.02),
				segment(Vec3::new(x, 0.5, 0.0), Vec3::new(0.1, 1.0, 0.0), 0.5, 0.02),
			])
		}));
		assert_eq!(collection.runs.len(), 4);
		assert_eq!(collection.runs[0].segments.len(), 2);

		let medium = collection.runs_for_level(LodSceneLevel::Medium);
		assert_eq!(medium.len(), 2);
		// Survivors keep both kink segments (connectivity preserved).
		assert_eq!(medium[0].segments.len(), 2);
		assert_eq!(medium[1].segments.len(), 2);

		let low = collection.runs_for_level(LodSceneLevel::Low);
		assert_eq!(low.len(), 1);
		assert_eq!(low[0].segments.len(), 1, "Low collapses each survivor to a chord");

		let ultra = collection.runs_for_level(LodSceneLevel::UltraLow);
		assert_eq!(ultra.len(), 1);
		assert_eq!(ultra[0].segments.len(), 1);
		Ok(())
	}

	#[test]
	fn medium_widens_by_absorbed_run_count() -> Result<()> {
		let collection =
			FrondCollection::new((0..4).map(|i| {
				run_of([segment(Vec3::new(i as f32 * 0.1, 0.0, 0.0), Vec3::Y, 1.0, 0.02)])
			}));
		let medium = collection.members_for_level(LodSceneLevel::Medium);
		assert_eq!(medium.len(), 2);
		let authored = Placement::frond_segment(Vec3::ZERO, Vec3::Y, 1.0, 0.02).unwrap().scale.x;
		assert!((medium[0].placement.scale.x - authored * 2.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn extent_uses_collection_aabb() -> Result<()> {
		let collection = FrondCollection::new([
			run_of([segment(Vec3::ZERO, Vec3::Y, 1.0, 0.02)]),
			run_of([segment(Vec3::new(2.0, 0.0, 0.0), Vec3::Y, 1.0, 0.02)]),
		]);
		let (center, extent) = collection.center_and_extent();
		assert!(center.x > 0.5 && center.x < 1.5);
		assert!(extent >= 1.0);
		Ok(())
	}
}
