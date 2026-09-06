//! Apply [`TerrainPitch`] from an [`ElevationProbe`]. No Durham types.
//!
//! Two downward rays (front / hind). Side rays only if `roll_weight > 0`.
//! The capsule stays upright and owns world Y. The visual child pitches and
//! lifts so the support chord stays on the slope.

use bevy::prelude::*;
use ground::ElevationProbe;

use crate::markers::{ApplyTerrainPitch, SuspendTerrainPitch};
use crate::pitch::{
	facing_with_tilt, observed_pitch, observed_roll, step_toward, support_lift, TerrainPitch,
	MAX_TILT,
};

/// Start the ray this far above the body so it clears the capsule.
pub const PROBE_LIFT: f32 = 2.0;
/// Extra downward reach after the uphill clearance, for downhill samples.
pub const PROBE_MAX_DISTANCE: f32 = 6.0;

/// Extra height so an uphill sample still starts above the mesh.
pub fn probe_clearance(half_span: f32) -> f32 {
	PROBE_LIFT + half_span.max(0.0) * MAX_TILT.tan()
}

fn probe_from_y(origin_y: f32, half_span: f32) -> f32 {
	origin_y + probe_clearance(half_span)
}

fn probe_distance(half_span: f32) -> f32 {
	probe_clearance(half_span) + PROBE_MAX_DISTANCE
}

/// Tilt visuals that carry both [`TerrainPitch`] and host [`ApplyTerrainPitch`].
///
/// `P` is typically [`ground_avian::AvianElevationProbe`]. Register after
/// physics / locomotion. Exclude the physics parent so the capsule is not a hit.
pub fn apply_terrain_pitch<P>(
	time: Res<Time>,
	mut probe: P,
	mut visuals: Query<
		(Entity, &mut Transform, &mut TerrainPitch, Option<&ChildOf>),
		With<ApplyTerrainPitch>,
	>,
	parents: Query<(&GlobalTransform, Has<SuspendTerrainPitch>)>,
) where
	P: ElevationProbe,
{
	let dt = time.delta_secs();
	for (_entity, mut visual, mut pitch, child_of) in &mut visuals {
		let facing = {
			let f = -visual.forward();
			Vec3::new(f.x, 0.0, f.z)
		};
		if facing.length_squared() < 1e-6 {
			continue;
		}
		let facing = facing.normalize();
		let right = Vec3::new(facing.z, 0.0, -facing.x);

		let (origin, exclude, suspend) = match child_of {
			Some(parent) => match parents.get(parent.parent()) {
				Ok((body, airborne)) => (body.translation(), vec![parent.parent()], airborne),
				Err(_) => (visual.translation, Vec::new(), false),
			},
			None => (visual.translation, Vec::new(), false),
		};

		let from_y = probe_from_y(origin.y, pitch.half_span);
		let max_distance = probe_distance(pitch.half_span);
		let mut sample = |xz: Vec3| probe.height_at(xz.x, xz.z, from_y, max_distance, &exclude);

		let center_h = sample(origin).unwrap_or(origin.y);
		let front_h = sample(origin + facing * pitch.half_span).unwrap_or(center_h);
		let hind_h = sample(origin - facing * pitch.half_span).unwrap_or(center_h);
		let (left_h, right_h) = if pitch.roll_weight > 0.0 {
			(
				sample(origin - right * pitch.half_width).unwrap_or(center_h),
				sample(origin + right * pitch.half_width).unwrap_or(center_h),
			)
		} else {
			(center_h, center_h)
		};

		let (target_pitch, target_roll) = if suspend {
			(0.0, 0.0)
		} else {
			(
				observed_pitch(front_h, hind_h, pitch.half_span) * pitch.pitch_weight,
				observed_roll(left_h, right_h, pitch.half_width) * pitch.roll_weight,
			)
		};
		pitch.pitch = step_toward(pitch.pitch, target_pitch, dt);
		pitch.roll = step_toward(pitch.roll, target_roll, dt);
		visual.rotation = facing_with_tilt(facing, pitch.pitch, pitch.roll);
		if child_of.is_some() {
			visual.translation.y = if suspend {
				0.0
			} else {
				support_lift(origin.y, center_h, front_h, hind_h, pitch.half_span, pitch.pitch)
			};
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn probe_clearance_starts_above_a_max_tilt_uphill_sample() {
		let span = 1.2;
		let clearance = probe_clearance(span);
		assert!(clearance > span * MAX_TILT.tan());
		assert!(probe_distance(span) > clearance);
	}
}
