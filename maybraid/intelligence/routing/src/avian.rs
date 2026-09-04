use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;

use crate::probe::RouteProbe;

/// Fixed-layer ground snaps and long hip chords.
pub struct AvianRouteProbe<'q, 'w, 's> {
	spatial: &'q SpatialQuery<'w, 's>,
	filter: SpatialQueryFilter,
}

impl<'q, 'w, 's> AvianRouteProbe<'q, 'w, 's> {
	pub fn new(spatial: &'q SpatialQuery<'w, 's>, exclude: &[Entity]) -> Self {
		Self {
			spatial,
			filter: SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)
				.with_excluded_entities(exclude.iter().copied()),
		}
	}
}

impl RouteProbe for AvianRouteProbe<'_, '_, '_> {
	fn ground(&self, xz: Vec2, hint_y: f32) -> Option<Vec3> {
		let lift = 8.0;
		let origin = Vec3::new(xz.x, hint_y + lift, xz.y);
		let hit = self.spatial.cast_ray(origin, Dir3::NEG_Y, 48.0, true, &self.filter)?;
		Some(Vec3::new(xz.x, origin.y - hit.distance, xz.y))
	}

	fn blocked(&self, from_hip: Vec3, to_hip: Vec3) -> bool {
		let delta = to_hip - from_hip;
		let dist = delta.length();
		if dist < 1e-4 {
			return false;
		}
		let Ok(direction) = Dir3::new(delta) else {
			return false;
		};
		match self.spatial.cast_ray(from_hip, direction, dist, true, &self.filter) {
			Some(hit) if hit.distance < dist - 0.2 => true,
			_ => false,
		}
	}
}
