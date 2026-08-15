//! Avian-backed [`ElevationProbe`](ground::ElevationProbe).
//!
//! Casts a downward ray through Avian [`SpatialQuery`]. Any collider in the
//! query (terrain, props, buildings) can support a character — this is not a
//! heightfield lookup.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::entity::Entity;
use bevy::ecs::system::SystemParam;
use bevy::math::{Dir3, Vec3};
use ground::{ElevationProbe, GroundHit};

/// [`SystemParam`] Avian implementation of [`ElevationProbe`].
///
/// Register `apply_terrain_pitch::<AvianElevationProbe>` (or any system that
/// takes this param) after physics so hits match this frame’s colliders.
#[derive(SystemParam)]
pub struct AvianElevationProbe<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
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
		let filter = if exclude.is_empty() {
			SpatialQueryFilter::default()
		} else {
			SpatialQueryFilter::from_excluded_entities(exclude.iter().copied())
		};
		let hit = self.spatial.cast_ray(origin, Dir3::NEG_Y, max_distance, true, &filter)?;
		Some(GroundHit {
			point: origin + Vec3::NEG_Y * hit.distance,
			normal: hit.normal,
			entity: hit.entity,
		})
	}
}
