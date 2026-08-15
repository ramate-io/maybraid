//! Apply [`TerrainPitch`] from an [`ElevationProbe`]. No Durham types.
//!
//! Two downward rays (front / hind). Side rays only if `roll_weight > 0`.
//! Rotation only — the physics capsule owns Y.

use bevy::prelude::*;
use ground::ElevationProbe;
use lod::{LodLevelRoot, LodLevelRoots};

use crate::markers::{ApplyTerrainPitch, SuspendTerrainPitch};
use crate::pitch::{facing_with_tilt, observed_pitch, observed_roll, step_toward, TerrainPitch};
use crate::shown::shown_level_has;

/// Start the ray this far above the body so it clears the capsule.
pub const PROBE_LIFT: f32 = 2.0;
/// Short downward cast — characters stand on nearby colliders, not a heightfield.
pub const PROBE_MAX_DISTANCE: f32 = 6.0;

/// Tilt each visual that has [`TerrainPitch`] when the shown LOD child (or the
/// host, before a level exists) carries [`ApplyTerrainPitch`].
///
/// `P` is typically [`ground_avian::AvianElevationProbe`]. Register after
/// physics / locomotion. Exclude the physics parent so the capsule is not a hit.
pub fn apply_terrain_pitch<P>(
	time: Res<Time>,
	mut probe: P,
	mut visuals: Query<(Entity, &mut Transform, &mut TerrainPitch, Option<&ChildOf>)>,
	parents: Query<(&GlobalTransform, Has<SuspendTerrainPitch>)>,
	children: Query<&Children>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	visibilities: Query<&Visibility>,
	apply_pitch: Query<(), With<ApplyTerrainPitch>>,
) where
	P: ElevationProbe,
{
	let dt = time.delta_secs();
	for (entity, mut visual, mut pitch, child_of) in &mut visuals {
		if !shown_level_has::<ApplyTerrainPitch>(
			entity,
			&children,
			&level_roots_bags,
			&root_keys,
			&visibilities,
			&apply_pitch,
		) {
			continue;
		}

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

		let mut sample = |xz: Vec3| {
			probe
				.height_at(xz.x, xz.z, origin.y + PROBE_LIFT, PROBE_MAX_DISTANCE, &exclude)
				.unwrap_or(origin.y)
		};

		let front_h = sample(origin + facing * pitch.half_span);
		let hind_h = sample(origin - facing * pitch.half_span);
		let (left_h, right_h) = if pitch.roll_weight > 0.0 {
			(sample(origin - right * pitch.half_width), sample(origin + right * pitch.half_width))
		} else {
			(origin.y, origin.y)
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
	}
}
