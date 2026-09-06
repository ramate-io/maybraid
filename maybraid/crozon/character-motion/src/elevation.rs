//! Apply [`TerrainPitch`] from an [`ElevationProbe`]. No Durham types.
//!
//! Two downward rays (front / hind). Side rays only if `roll_weight > 0`.
//! The capsule stays upright and owns world Y. The visual child pitches about
//! the origin and takes a signed Y offset so the chord midpoint matches the
//! sampled slope, without max-lifting the rear into the air.
//!
//! Front/hind rays follow [`TerrainPitch::sagittal`] (live shoulder–hip) when
//! that axis is set, otherwise Bevy mesh `+Z`. Yaw still comes from the visual
//! so locomotion keeps facing.

use bevy::prelude::*;
use ground::ElevationProbe;

use crate::markers::{ApplyTerrainPitch, SuspendTerrainPitch};
use crate::pitch::{
	facing_with_support_tilt, observed_pitch, observed_roll, pitched_half_run, sample_facing,
	step_toward, support_offset, xz_dir, TerrainPitch, TerrainPitchProbe, MAX_TILT,
};

/// Start the ray this far above the body so it clears the capsule.
pub const PROBE_LIFT: f32 = 2.0;
/// Extra downward reach after the uphill clearance, for downhill samples.
pub const PROBE_MAX_DISTANCE: f32 = 6.0;

/// Draw last-frame front/hind hits. Playgrounds leave this on; set `false` to hide.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawTerrainPitchProbes(pub bool);

impl Default for DrawTerrainPitchProbes {
	fn default() -> Self {
		Self(true)
	}
}

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

fn visual_plus_z(visual: &Transform) -> Option<Vec3> {
	xz_dir(-*visual.forward())
}

/// Local visual under a capsule sits near the parent origin. World-placed hosts
/// keep large local XZ when parented to an identity cell/group root.
pub fn is_local_visual_child(local_translation: Vec3) -> bool {
	Vec2::new(local_translation.x, local_translation.z).length_squared() < 4.0
}

fn ancestor_exclude(
	start: Option<Entity>,
	child_of: &Query<&ChildOf>,
	suspended: &Query<(), With<SuspendTerrainPitch>>,
) -> (Vec<Entity>, bool) {
	let mut exclude = Vec::new();
	let mut airborne = false;
	let mut current = start;
	for _ in 0..32 {
		let Some(entity) = current else {
			break;
		};
		exclude.push(entity);
		airborne |= suspended.contains(entity);
		current = child_of.get(entity).ok().map(ChildOf::parent);
	}
	(exclude, airborne)
}

/// Tilt visuals that carry both [`TerrainPitch`] and host [`ApplyTerrainPitch`].
///
/// `P` is typically [`ground_avian::AvianElevationProbe`]. Register after
/// physics / locomotion. Sample from the visual's world pose. Exclude every
/// ancestor so a cell/group root is never treated as the capsule.
pub fn apply_terrain_pitch<P>(
	time: Res<Time>,
	mut probe: P,
	mut visuals: Query<
		(Entity, &mut Transform, &GlobalTransform, &mut TerrainPitch),
		With<ApplyTerrainPitch>,
	>,
	child_of: Query<&ChildOf>,
	suspended: Query<(), With<SuspendTerrainPitch>>,
) where
	P: ElevationProbe,
{
	let dt = time.delta_secs();
	for (entity, mut visual, global, mut pitch) in &mut visuals {
		let Some(visual_facing) = visual_plus_z(&visual) else {
			continue;
		};
		let ray_facing = sample_facing(pitch.sagittal, visual_facing);
		let right = Vec3::new(visual_facing.z, 0.0, -visual_facing.x);

		let origin = global.translation();
		let (exclude, suspend) = ancestor_exclude(Some(entity), &child_of, &suspended);
		let offset_local_y =
			child_of.get(entity).is_ok() && is_local_visual_child(visual.translation);

		let from_y = probe_from_y(origin.y, pitch.half_span);
		let max_distance = probe_distance(pitch.half_span);
		let mut sample = |xz: Vec3| -> (Vec3, bool) {
			match probe.height_at(xz.x, xz.z, from_y, max_distance, &exclude) {
				Some(h) => (Vec3::new(xz.x, h, xz.z), true),
				None => (Vec3::new(xz.x, origin.y, xz.z), false),
			}
		};

		let (center, _) = sample(origin);
		let center_h = center.y;
		let mut run = pitch.half_span;
		let mut front = center;
		let mut hind = center;
		let mut front_hit = false;
		let mut hind_hit = false;
		let mut left_h = center_h;
		let mut right_h = center_h;
		if !suspend {
			let (coarse_front, _) = sample(origin + ray_facing * pitch.half_span);
			let (coarse_hind, _) = sample(origin - ray_facing * pitch.half_span);
			let coarse = observed_pitch(coarse_front.y, coarse_hind.y, pitch.half_span);
			run = pitched_half_run(pitch.half_span, coarse);
			(front, front_hit) = sample(origin + ray_facing * run);
			(hind, hind_hit) = sample(origin - ray_facing * run);
			if pitch.roll_weight > 0.0 {
				left_h = sample(origin - right * pitch.half_width).0.y;
				right_h = sample(origin + right * pitch.half_width).0.y;
			}
		}

		let (target_pitch, target_roll) = if suspend {
			(0.0, 0.0)
		} else {
			(
				observed_pitch(front.y, hind.y, run) * pitch.pitch_weight,
				observed_roll(left_h, right_h, pitch.half_width) * pitch.roll_weight,
			)
		};
		pitch.pitch = step_toward(pitch.pitch, target_pitch, dt);
		pitch.roll = step_toward(pitch.roll, target_roll, dt);
		pitch.probe = TerrainPitchProbe {
			origin: Vec3::new(origin.x, center_h, origin.z),
			front,
			hind,
			front_hit,
			hind_hit,
			visual_facing,
			sample_facing: ray_facing,
		};
		visual.rotation = facing_with_support_tilt(visual_facing, ray_facing, pitch.pitch, pitch.roll);
		if offset_local_y {
			visual.translation.y = if suspend {
				0.0
			} else {
				support_offset(center_h, front.y, hind.y)
			};
		}
	}
}

/// Front = lime, hind = orange, misses = red. Yellow = sample axis, cyan = mesh `+Z`.
/// Magenta = live girdle bones/midpoints. A red ring means girdles were seen but
/// the XZ run was too short to accept as a sagittal axis.
pub fn draw_terrain_pitch_probes(
	draw: Option<Res<DrawTerrainPitchProbes>>,
	mut gizmos: Gizmos,
	pitched: Query<&TerrainPitch, With<ApplyTerrainPitch>>,
) {
	if !draw.map(|d| d.0).unwrap_or(true) {
		return;
	}
	for pitch in &pitched {
		draw_girdles(&mut gizmos, pitch);
		let probe = pitch.probe;
		if probe.sample_facing.length_squared() < 1e-6 {
			continue;
		}
		let span = pitch.half_span.max(0.2);
		let origin = probe.origin;
		gizmos.line(
			origin - probe.sample_facing * span,
			origin + probe.sample_facing * span,
			Color::srgb(1.0, 0.9, 0.15),
		);
		gizmos.line(
			origin,
			origin + probe.visual_facing * span,
			Color::srgb(0.25, 0.75, 1.0),
		);
		draw_probe(&mut gizmos, probe.front, probe.front_hit, Color::srgb(0.25, 0.95, 0.35));
		draw_probe(&mut gizmos, probe.hind, probe.hind_hit, Color::srgb(0.95, 0.5, 0.12));
		gizmos.sphere(Isometry3d::from_translation(origin), 0.08, Color::srgb(1.0, 1.0, 1.0));
	}
}

fn draw_girdles(gizmos: &mut Gizmos, pitch: &TerrainPitch) {
	let g = pitch.girdles;
	let bone = Color::srgb(0.85, 0.2, 1.0);
	let chord = Color::srgb(1.0, 0.35, 0.85);
	for p in [g.shoulder_l, g.shoulder_r, g.hip_l, g.hip_r].into_iter().flatten() {
		gizmos.sphere(Isometry3d::from_translation(p), 0.1, bone);
		gizmos.line(p, p + Vec3::Y * 0.45, bone);
	}
	if let (Some(front), Some(hind)) = (g.front, g.hind) {
		gizmos.line(front, hind, chord);
		gizmos.sphere(Isometry3d::from_translation(front), 0.16, Color::srgb(0.4, 1.0, 0.85));
		gizmos.sphere(Isometry3d::from_translation(hind), 0.16, Color::srgb(1.0, 0.7, 0.2));
	}
	if !g.sagittal_ok {
		if let Some(at) = g.front.or(g.hind).or(g.shoulder_l).or(g.hip_l) {
			gizmos.sphere(Isometry3d::from_translation(at), 0.28, Color::srgb(0.95, 0.2, 0.55));
		}
	}
}

fn draw_probe(gizmos: &mut Gizmos, at: Vec3, hit: bool, hit_color: Color) {
	let color = if hit { hit_color } else { Color::srgb(0.95, 0.15, 0.15) };
	let radius = if hit { 0.14 } else { 0.2 };
	gizmos.sphere(Isometry3d::from_translation(at), radius, color);
	gizmos.line(at, at + Vec3::Y * 0.6, color);
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

	#[test]
	fn capsule_child_offsets_local_y_world_host_does_not() {
		assert!(is_local_visual_child(Vec3::ZERO));
		assert!(is_local_visual_child(Vec3::new(0.1, 0.0, -0.2)));
		assert!(!is_local_visual_child(Vec3::new(40.0, 12.0, -8.0)));
	}
}
