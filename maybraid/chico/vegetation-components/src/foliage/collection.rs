//! Frond collection: placed fronds with extent-based merge LOD.

use bevy::prelude::{Component, Query, Res, Transform, With};
use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;

use crate::foliage::node::FoliageNode;
use crate::lod_band::{warm_mesh_lod_culls, DistanceLodBand};
use crate::lod_host::warm_content_host;
use crate::procedural::FROND_KIT_HALF_X;
use crate::scene_children::scene_children;

/// `distance / collection_extent` at or below this → High (full frond set).
pub const FROND_COLLECTION_HIGH_FACTOR: f32 = 5.0;
/// At or below this → Medium (first merge).
pub const FROND_COLLECTION_MEDIUM_FACTOR: f32 = 20.0;
/// At or below this → Low (further merge); beyond → UltraLow (single marker frond).
pub const FROND_COLLECTION_LOW_FACTOR: f32 = 80.0;

/// A set of placed fronds that share one LOD host and merge as distance grows.
///
/// Bands use the collection's **max extent** (half the longest AABB axis). Merge
/// drops entries and scales surviving blade widths so silhouette mass stays roughly
/// constant: High = all; Medium ≈ half; Low ≈ quarter; UltraLow = one marker.
#[derive(Debug, Clone, PartialEq)]
pub struct FrondCollection {
	pub fronds: Vec<FoliageNode>,
}

impl FrondCollection {
	pub fn new(fronds: impl IntoIterator<Item = FoliageNode>) -> Self {
		Self { fronds: fronds.into_iter().collect() }
	}

	pub fn is_empty(&self) -> bool {
		self.fronds.is_empty()
	}

	/// Axis-aligned bounds of all frond bases/tips expanded by blade half-width.
	pub fn aabb(&self) -> Option<(Vec3, Vec3)> {
		if self.fronds.is_empty() {
			return None;
		}
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		for frond in &self.fronds {
			let base = frond.placement.translation;
			let tip = base
				+ frond.placement.rotation() * Vec3::new(0.0, frond.placement.scale.y.abs(), 0.0);
			let half_w = (frond.placement.scale.x.abs() * FROND_KIT_HALF_X).max(1e-4);
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

	pub fn probe(&self) -> FrondCollectionLodProbe {
		let (center, extent) = self.center_and_extent();
		FrondCollectionLodProbe { center, extent }
	}

	/// Fronds presented at `level` after merge thinning.
	pub fn fronds_for_level(&self, level: LodSceneLevel) -> Vec<FoliageNode> {
		let n = self.fronds.len();
		if n == 0 {
			return Vec::new();
		}
		let target = match level {
			LodSceneLevel::High => n,
			LodSceneLevel::Medium => (n.div_ceil(2)).max(1),
			LodSceneLevel::Low => (n.div_ceil(4)).max(1),
			LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => 1,
		};
		merge_fronds(&self.fronds, target)
	}

	fn content_for_level(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		let children: Vec<Box<dyn Scene>> = self
			.fronds_for_level(level)
			.into_iter()
			.map(|frond| frond.collection_leaf_scene(level))
			.collect();
		Box::new(scene_children(children))
	}
}

/// Merge `fronds` down to `target` survivors: absorb neighbors into the longest
/// blade of each group and scale its \(X/Z\) width by the group size.
fn merge_fronds(fronds: &[FoliageNode], target: usize) -> Vec<FoliageNode> {
	let n = fronds.len();
	if n == 0 {
		return Vec::new();
	}
	let target = target.clamp(1, n);
	if target == n {
		return fronds.to_vec();
	}
	let mut out = Vec::with_capacity(target);
	for k in 0..target {
		let start = (k * n) / target;
		let end = ((k + 1) * n) / target;
		let group = &fronds[start..end];
		let factor = (group.len() as f32).max(1.0);
		let mut best = group[0].clone();
		for candidate in group.iter().skip(1) {
			if candidate.placement.scale.y.abs() > best.placement.scale.y.abs() {
				best = candidate.clone();
			}
		}
		best.placement.scale.x *= factor;
		best.placement.scale.z *= factor;
		out.push(best);
	}
	out
}

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct FrondCollectionLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl FrondCollectionLodProbe {
	fn band_to_level(band: DistanceLodBand) -> LodSceneLevel {
		match band {
			DistanceLodBand::High => LodSceneLevel::High,
			DistanceLodBand::Medium => LodSceneLevel::Medium,
			DistanceLodBand::Low => LodSceneLevel::Low,
			// Real UltraLow tier (single marker), not collapsed onto Low.
			DistanceLodBand::UltraLow => LodSceneLevel::UltraLow,
		}
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		let factor = viewer.translation.distance(self.center) / self.extent.max(1e-4);
		Self::band_to_level(DistanceLodBand::from_factors(
			factor,
			FROND_COLLECTION_HIGH_FACTOR,
			FROND_COLLECTION_MEDIUM_FACTOR,
			FROND_COLLECTION_LOW_FACTOR,
		))
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = Self::band_to_level(DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center) / self.extent.max(1e-4),
			FROND_COLLECTION_HIGH_FACTOR,
			FROND_COLLECTION_MEDIUM_FACTOR,
			FROND_COLLECTION_LOW_FACTOR,
		));
		let curr = Self::band_to_level(DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center) / self.extent.max(1e-4),
			FROND_COLLECTION_HIGH_FACTOR,
			FROND_COLLECTION_MEDIUM_FACTOR,
			FROND_COLLECTION_LOW_FACTOR,
		));
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}
}

pub fn update_frond_collection_host_levels(
	lod_state: Res<lod::LodViewerState>,
	mut hosts: Query<(&FrondCollectionLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let viewer = lod_state.current;
	for (probe, mut level) in &mut hosts {
		let next = probe.level_for(&viewer);
		if *level != next {
			*level = next;
		}
	}
}

impl LodScene for FrondCollection {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		let probe = self.probe();
		warm_content_host(
			level,
			probe,
			[
				(LodSceneLevel::High, self.content_for_level(LodSceneLevel::High)),
				(LodSceneLevel::Medium, self.content_for_level(LodSceneLevel::Medium)),
				(LodSceneLevel::Low, self.content_for_level(LodSceneLevel::Low)),
				(
					LodSceneLevel::UltraLow,
					self.content_for_level(LodSceneLevel::UltraLow),
				),
			],
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::placed::Placement;
	use anyhow::Result;

	fn segment(start: Vec3, dir: Vec3, length: f32, width: f32) -> FoliageNode {
		FoliageNode::straight_frond_segment(
			Placement::frond_segment(start, dir, length, width).expect("placement"),
		)
	}

	#[test]
	fn merge_halves_count_and_scales_width() -> Result<()> {
		let fronds: Vec<_> = (0..4)
			.map(|i| {
				segment(
					Vec3::new(i as f32 * 0.1, 0.0, 0.0),
					Vec3::Y,
					1.0,
					0.02,
				)
			})
			.collect();
		let collection = FrondCollection::new(fronds);
		let medium = collection.fronds_for_level(LodSceneLevel::Medium);
		assert_eq!(medium.len(), 2);
		// Each survivor absorbs 2 → width scale ×2 relative to authored girth scale.
		let authored_scale = Placement::frond_segment(Vec3::ZERO, Vec3::Y, 1.0, 0.02)
			.unwrap()
			.scale
			.x;
		assert!((medium[0].placement.scale.x - authored_scale * 2.0).abs() < 1e-4);
		let ultra = collection.fronds_for_level(LodSceneLevel::UltraLow);
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
