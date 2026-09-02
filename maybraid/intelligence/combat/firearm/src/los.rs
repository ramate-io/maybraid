//! Shared sightline probes for spotting and fire obstruction.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

/// Skip this much of a probe so an origin clipped into nearby Fixed geometry
/// does not report an immediate hit and go blind.
pub(crate) const RAY_ORIGIN_SKIP: f32 = 0.12;

pub(crate) fn clear_segment(
	start: Vec3,
	end: Vec3,
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
) -> bool {
	let delta = end - start;
	let distance = delta.length();
	if distance <= RAY_ORIGIN_SKIP + 1e-4 {
		return true;
	}
	let Ok(direction) = Dir3::new(delta) else {
		return true;
	};
	let origin = start + *direction * RAY_ORIGIN_SKIP;
	let remain = distance - RAY_ORIGIN_SKIP;
	// `solid: false` so an origin already inside Fixed geometry (muzzle in a
	// pillar, eyes clipped into a wall) is not an instant hit. The next surface
	// along the segment still blocks.
	spatial
		.cast_ray(origin, direction, remain, false, filter)
		.is_none_or(|hit| hit.distance >= remain - 0.05)
}
