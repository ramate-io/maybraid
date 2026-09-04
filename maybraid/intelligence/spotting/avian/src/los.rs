use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

/// Skip this much of a segment so an eye clipped into nearby geometry does not
/// report an immediate obstruction.
pub const RAY_ORIGIN_SKIP: f32 = 0.12;

/// Whether a finite segment has no blocking hit under `filter`.
///
/// `solid` is false so a start point already inside geometry is not treated as
/// an immediate hit. The next surface along the segment still blocks.
pub fn clear_segment(
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
	spatial
		.cast_ray(origin, direction, remain, false, filter)
		.is_none_or(|hit| hit.distance >= remain - 0.05)
}
