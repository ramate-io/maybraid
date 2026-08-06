//! Frond collection geometry — many placed frond kits under one foliage LOD parent.
//!
//! Analogous to polyline / tessellated partitions in Richmond building-components:
//! the continuous form is a [`FoliageGeometry::FrondCollection`](crate::FoliageGeometry)
//! on a single [`FoliageNode`](crate::FoliageNode); leaf kits are expanded per LOD band.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;

use crate::placed::Placement;
use crate::procedural::FROND_KIT_HALF_X;

/// Collection LOD bands: `distance / collection_max_extent`.
///
/// Defined here (used by [`crate::FoliageLodProbe::for_frond_collection`]):
/// - High when factor ≤ [`FROND_COLLECTION_HIGH_FACTOR`]
/// - Medium when factor ≤ [`FROND_COLLECTION_MEDIUM_FACTOR`] (first merge ≈ half strands)
/// - Low when factor ≤ [`FROND_COLLECTION_LOW_FACTOR`] (further merge ≈ quarter)
/// - UltraLow beyond that (single marker frond; real [`LodSceneLevel::UltraLow`] tier)
pub const FROND_COLLECTION_HIGH_FACTOR: f32 = 5.0;
/// See [`FROND_COLLECTION_HIGH_FACTOR`].
pub const FROND_COLLECTION_MEDIUM_FACTOR: f32 = 18.0;
/// See [`FROND_COLLECTION_HIGH_FACTOR`].
pub const FROND_COLLECTION_LOW_FACTOR: f32 = 50.0;

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

/// One placed frond leaf inside a [`FrondCollection`].
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
}

/// Continuous frond-collection form: many leaf kits, one LOD parent.
///
/// Merge drops members and scales surviving blade widths so silhouette mass stays
/// roughly constant: High = all; Medium ≈ half; Low ≈ quarter; UltraLow = one marker.
#[derive(Debug, Clone, PartialEq)]
pub struct FrondCollection {
	pub members: Vec<FrondMember>,
}

impl FrondCollection {
	pub fn new(members: impl IntoIterator<Item = FrondMember>) -> Self {
		Self { members: members.into_iter().collect() }
	}

	pub fn segments(placements: impl IntoIterator<Item = Placement>) -> Self {
		Self::new(placements.into_iter().map(FrondMember::segment))
	}

	pub fn is_empty(&self) -> bool {
		self.members.is_empty()
	}

	/// Axis-aligned bounds of all member bases/tips expanded by blade half-width.
	pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
		if self.members.is_empty() {
			return None;
		}
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		for member in &self.members {
			let base = member.placement.translation;
			let tip = base
				+ member.placement.rotation() * Vec3::new(0.0, member.placement.scale.y.abs(), 0.0);
			let half_w = (member.placement.scale.x.abs() * FROND_KIT_HALF_X).max(1e-4);
			for p in [base, tip] {
				min = min.min(p - Vec3::splat(half_w));
				max = max.max(p + Vec3::splat(half_w));
			}
		}
		Some((min, max))
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

	/// Members presented at `level` after merge thinning.
	pub fn members_for_level(&self, level: LodSceneLevel) -> Vec<FrondMember> {
		let n = self.members.len();
		if n == 0 {
			return Vec::new();
		}
		let target = match level {
			LodSceneLevel::High => n,
			LodSceneLevel::Medium => n.div_ceil(2).max(1),
			LodSceneLevel::Low => n.div_ceil(4).max(1),
			LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
				1
			}
		};
		merge_members(&self.members, target)
	}
}

/// Merge `members` down to `target` survivors: absorb neighbors into the longest
/// blade of each group and scale its \(X/Z\) width by the group size.
fn merge_members(members: &[FrondMember], target: usize) -> Vec<FrondMember> {
	let n = members.len();
	if n == 0 {
		return Vec::new();
	}
	let target = target.clamp(1, n);
	if target == n {
		return members.to_vec();
	}
	let mut out = Vec::with_capacity(target);
	for k in 0..target {
		let start = (k * n) / target;
		let end = ((k + 1) * n) / target;
		let group = &members[start..end];
		let factor = (group.len() as f32).max(1.0);
		let mut best = group[0];
		for candidate in group.iter().skip(1) {
			if candidate.placement.scale.y.abs() > best.placement.scale.y.abs() {
				best = *candidate;
			}
		}
		best.placement.scale.x *= factor;
		best.placement.scale.z *= factor;
		out.push(best);
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

	#[test]
	fn merge_halves_count_and_scales_width() -> Result<()> {
		let collection = FrondCollection::new(
			(0..4).map(|i| segment(Vec3::new(i as f32 * 0.1, 0.0, 0.0), Vec3::Y, 1.0, 0.02)),
		);
		let medium = collection.members_for_level(LodSceneLevel::Medium);
		assert_eq!(medium.len(), 2);
		let authored_scale =
			Placement::frond_segment(Vec3::ZERO, Vec3::Y, 1.0, 0.02).unwrap().scale.x;
		assert!((medium[0].placement.scale.x - authored_scale * 2.0).abs() < 1e-4);
		let ultra = collection.members_for_level(LodSceneLevel::UltraLow);
		assert_eq!(ultra.len(), 1);
		assert!((ultra[0].placement.scale.x - authored_scale * 4.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn extent_uses_collection_aabb() -> Result<()> {
		let collection = FrondCollection::new([
			segment(Vec3::ZERO, Vec3::Y, 1.0, 0.02),
			segment(Vec3::new(2.0, 0.0, 0.0), Vec3::Y, 1.0, 0.02),
		]);
		let (center, extent) = collection.center_and_extent();
		assert!(center.x > 0.5 && center.x < 1.5);
		assert!(extent >= 1.0);
		Ok(())
	}
}
