//! Avian-backed [`ElevationProbe`](ground::ElevationProbe).
//!
//! Walks a downward column through [`PhysicsInteractionLayer::Fixed`] colliders
//! (terrain trimeshes, buildings, tree stick compounds). LOD Host / Generate /
//! Present volumes and Animated capsules are ignored. Near-start hits are
//! skipped so a ray that begins inside a canopy or solid AABB does not count as
//! ground; among the rest, the **lowest** hit wins so a tree canopy cannot
//! steal the Durham trimesh.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::entity::Entity;
use bevy::ecs::system::SystemParam;
use bevy::math::{Dir3, Vec3};
use ground::{ElevationProbe, GroundHit};
use lod_avian::PhysicsInteractionLayer;

/// Matches `crozon_character_motion::PROBE_LIFT`: pitch rays start this far
/// above the body. Hits closer than this are canopy / solid-start volumes.
pub const MIN_GROUND_DROP: f32 = 2.0;

/// Grove tiles and overlapping High-band plants can stack several Fixed
/// volumes on one plumb line. Walk past them to the trimesh.
const MAX_COLUMN_HITS: usize = 8;

/// [`SystemParam`] Avian implementation of [`ElevationProbe`].
///
/// Register `apply_terrain_pitch::<AvianElevationProbe>` (or any system that
/// takes this param) after physics so hits match this frame’s colliders.
#[derive(SystemParam)]
pub struct AvianElevationProbe<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

fn fixed_filter(exclude: impl IntoIterator<Item = Entity>) -> SpatialQueryFilter {
	SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed).with_excluded_entities(exclude)
}

impl ElevationProbe for AvianElevationProbe<'_, '_> {
	fn hit_down(
		&mut self,
		origin: Vec3,
		max_distance: f32,
		exclude: &[Entity],
	) -> Option<GroundHit> {
		if max_distance <= 0.0 {
			return None;
		}
		let mut skipped: Vec<Entity> = exclude.to_vec();
		let mut best: Option<GroundHit> = None;
		for _ in 0..MAX_COLUMN_HITS {
			let filter = fixed_filter(skipped.iter().copied());
			let Some(hit) = self
				.spatial
				.cast_ray(origin, Dir3::NEG_Y, max_distance, true, &filter)
			else {
				break;
			};
			skipped.push(hit.entity);
			if hit.distance < MIN_GROUND_DROP {
				continue;
			}
			best = Some(GroundHit {
				point: origin + Vec3::NEG_Y * hit.distance,
				normal: hit.normal,
				entity: hit.entity,
			});
		}
		best
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Farthest (lowest) sample at least `min_drop` below the ray origin.
	fn select_column_distance(distances: &[f32], min_drop: f32) -> Option<f32> {
		distances
			.iter()
			.copied()
			.filter(|distance| *distance >= min_drop)
			.max_by(f32::total_cmp)
	}

	#[test]
	fn column_keeps_the_lowest_hit_past_canopy() {
		assert_eq!(
			select_column_distance(&[0.4, 2.8, 9.1], MIN_GROUND_DROP),
			Some(9.1)
		);
	}

	#[test]
	fn column_ignores_only_near_start_hits() {
		assert_eq!(select_column_distance(&[0.4, 1.9], MIN_GROUND_DROP), None);
		assert_eq!(select_column_distance(&[8.5], MIN_GROUND_DROP), Some(8.5));
	}
}
