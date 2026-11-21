//! CPU mirror of `parting.wesl`.

use bevy::math::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};

use crate::wesl::hash::hash_u32_to_unit;

pub const PARTING_MODE_OFF: u32 = 0;
pub const PARTING_MODE_DEPTH: u32 = 1;
pub const PARTING_MODE_RASTER: u32 = 2;
pub const PARTING_MODE_WORLD: u32 = 3;

/// Envelope along the primitive. Roots (`t ≈ 0`) stay pinned.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartingEnvelope {
	pub weight: f32,
}

impl PartingEnvelope {
	pub fn along(t: f32, stiffness: f32) -> Self {
		Self { weight: parting_along(t, stiffness) }
	}
}

pub fn parting_along(t: f32, stiffness: f32) -> f32 {
	let along = t.clamp(0.0, 1.0);
	along * along * (1.0 - stiffness).clamp(0.0, 1.0)
}

pub fn max_parting_displacement_m(height_m: f32, strength_m: f32, stiffness: f32) -> f32 {
	height_m * strength_m * (1.0 - stiffness).clamp(0.0, 1.0)
}

pub fn depth_gap_weight(blade_dist_m: f32, occluder_dist_m: f32, falloff_m: f32) -> f32 {
	let gap = occluder_dist_m - blade_dist_m;
	if gap <= 1e-3 {
		return 0.0;
	}
	let falloff = falloff_m.max(1e-3);
	let enter = smoothstep(0.0, 0.08, gap);
	let leave = 1.0 - smoothstep(falloff * 0.35, falloff, gap);
	(enter * leave).clamp(0.0, 1.0)
}

pub fn ground_occluder_weight(occluder_y: f32, root_y: f32, ground_reject_m: f32) -> f32 {
	smoothstep(root_y + ground_reject_m * 0.35, root_y + ground_reject_m, occluder_y)
}

pub fn clip_to_uv(clip: Vec4) -> Vec2 {
	let w = clip.w.abs().max(1e-5);
	let ndc = Vec2::new(clip.x / w, clip.y / w);
	ndc * Vec2::new(0.5, -0.5) + Vec2::splat(0.5)
}

pub fn uv_to_ndc(uv: Vec2, depth: f32) -> Vec3 {
	Vec3::new(uv.x * 2.0 - 1.0, -(uv.y * 2.0 - 1.0), depth)
}

pub fn reconstruct_world(uv: Vec2, depth: f32, world_from_clip: Mat4) -> Vec3 {
	let clip = uv_to_ndc(uv, depth).extend(1.0);
	let world = world_from_clip * clip;
	world.truncate() / world.w.max(1e-5)
}

pub fn screen_part_direction(gradient: Vec2, plant_id: u32) -> Vec2 {
	let len = gradient.length();
	if len > 0.045 {
		return -gradient / len;
	}
	let side = if hash_u32_to_unit(plant_id ^ 0xc2b2_ae35) >= 0.5 { 1.0 } else { -1.0 };
	Vec2::new(side, 0.0)
}

pub fn world_part_from_screen(
	dir_ss: Vec2,
	camera_right: Vec3,
	camera_up: Vec3,
	camera_forward: Vec3,
) -> Vec3 {
	let mut world = camera_right * dir_ss.x + camera_up * dir_ss.y;
	world -= camera_forward * world.dot(camera_forward);
	world.y = 0.0;
	let len = world.length();
	if len < 1e-5 {
		let mut fallback = camera_right;
		fallback.y = 0.0;
		return fallback.normalize_or(Vec3::X);
	}
	world / len
}

pub fn capsule_behind_weight(
	root: Vec3,
	camera: Vec3,
	occluder_center: Vec3,
	camera_forward: Vec3,
	falloff_m: f32,
) -> f32 {
	let blade_along = (root - camera).dot(camera_forward);
	let occ_along = (occluder_center - camera).dot(camera_forward);
	depth_gap_weight(blade_along, occ_along, falloff_m)
}

pub fn capsule_lateral_xz(root_xz: Vec2, occluder_xz: Vec2, forward_xz: Vec2) -> Vec2 {
	let fwd = forward_xz.normalize_or(Vec2::Y);
	let to_root = root_xz - occluder_xz;
	to_root - fwd * to_root.dot(fwd)
}

pub fn capsule_away_xz(
	root_xz: Vec2,
	occluder_xz: Vec2,
	camera_right_xz: Vec2,
	forward_xz: Vec2,
) -> Vec2 {
	let away = capsule_lateral_xz(root_xz, occluder_xz, forward_xz);
	let dist = away.length();
	if dist < 1e-4 {
		return camera_right_xz.normalize_or(Vec2::X);
	}
	away / dist
}

pub fn capsule_radial_weight(
	root_xz: Vec2,
	occluder_xz: Vec2,
	forward_xz: Vec2,
	radius_m: f32,
) -> f32 {
	let dist = capsule_lateral_xz(root_xz, occluder_xz, forward_xz).length();
	let influence = radius_m.max(1e-3);
	1.0 - smoothstep(influence * 0.35, influence, dist)
}

/// Keep the longer of two pre-envelope XZ offsets.
pub fn stronger_part_xz(a: Vec2, b: Vec2) -> Vec2 {
	if a.length_squared() >= b.length_squared() {
		a
	} else {
		b
	}
}

pub fn part_along_curve(rest: Vec3, root: Vec3, t: f32, part_xz: Vec2, stiffness: f32) -> Vec3 {
	let along = t.clamp(0.0, 1.0);
	if along <= 1e-4 {
		return root;
	}
	let weight = parting_along(along, stiffness);
	rest + Vec3::new(part_xz.x * weight, -part_xz.length() * weight * 0.06, part_xz.y * weight)
}

/// Raster stamp lean: `t²` along the blade, length-preserving, away from centre.
pub fn part_along_length(rest: Vec3, root: Vec3, t: f32, part_xz: Vec2, stiffness: f32) -> Vec3 {
	let along = t.clamp(0.0, 1.0);
	if along <= 1e-4 {
		return root;
	}
	let from_root = rest - root;
	let rest_len = from_root.length();
	if rest_len < 1e-5 {
		return root;
	}
	let lean_m = part_xz.length() * parting_along(along, stiffness);
	if lean_m < 1e-6 {
		return rest;
	}
	let away = part_xz.normalize_or(Vec2::X);
	let bent = from_root + Vec3::new(away.x, 0.0, away.y) * lean_m;
	let bent_len = bent.length();
	if bent_len < 1e-5 {
		return rest;
	}
	root + bent * (rest_len / bent_len)
}

/// One world-space capsule used by the CPU mirror and playground tests.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartingCapsule {
	pub center: Vec3,
	pub radius_m: f32,
	pub height_m: f32,
}

impl PartingCapsule {
	pub fn character(center: Vec3) -> Self {
		Self { center, radius_m: 0.4, height_m: 1.8 }
	}

	pub fn rock(center: Vec3, radius_m: f32) -> Self {
		Self { center, radius_m, height_m: radius_m * 2.0 }
	}

	pub fn influence_radius(self, radius_scale: f32) -> f32 {
		self.radius_m + radius_scale.max(0.0)
	}

	/// View-dependent world-space parting at a blade root. XZ metres, pre-envelope.
	pub fn sample_world(
		self,
		root: Vec3,
		camera: Vec3,
		camera_forward: Vec3,
		camera_right: Vec3,
		radius_scale: f32,
		falloff_m: f32,
		ground_reject_m: f32,
	) -> Vec2 {
		let behind = capsule_behind_weight(root, camera, self.center, camera_forward, falloff_m);
		let ground = ground_occluder_weight(self.center.y, root.y, ground_reject_m);
		let radial = capsule_radial_weight(
			root.xz(),
			self.center.xz(),
			camera_forward.xz(),
			self.influence_radius(radius_scale),
		);
		let weight = behind * ground * radial;
		if weight <= 1e-5 {
			return Vec2::ZERO;
		}
		capsule_away_xz(root.xz(), self.center.xz(), camera_right.xz(), camera_forward.xz())
			* weight
	}
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn roots_stay_pinned() -> anyhow::Result<()> {
		let root = Vec3::new(2.0, 0.1, -3.0);
		let rest = root + Vec3::Y * 0.01;
		let parted = part_along_curve(rest, root, 0.0, Vec2::new(1.5, 0.4), 0.0);
		assert_eq!(parted, root);
		Ok(())
	}

	#[test]
	fn tip_deflects_more_than_mid() -> anyhow::Result<()> {
		let root = Vec3::ZERO;
		let rest_mid = Vec3::new(0.0, 0.3, 0.0);
		let rest_tip = Vec3::new(0.0, 0.6, 0.0);
		let part = Vec2::new(0.4, 0.0);
		let mid = part_along_curve(rest_mid, root, 0.5, part, 0.0);
		let tip = part_along_curve(rest_tip, root, 1.0, part, 0.0);
		assert!((tip.x - rest_tip.x).abs() > (mid.x - rest_mid.x).abs());
		Ok(())
	}

	#[test]
	fn raster_length_bend_pins_the_root() -> anyhow::Result<()> {
		let root = Vec3::new(1.2, 0.05, -0.4);
		let rest = root + Vec3::Y * 0.02;
		let parted = part_along_length(rest, root, 0.0, Vec2::new(1.4, 0.3), 0.0);
		assert_eq!(parted, root);
		Ok(())
	}

	#[test]
	fn raster_length_bend_grows_monotonically_along_the_blade() -> anyhow::Result<()> {
		let root = Vec3::ZERO;
		let part = Vec2::new(0.85, 0.0);
		let mut previous = 0.0;
		for step in 1..=8 {
			let t = step as f32 / 8.0;
			let rest = root + Vec3::Y * (1.12 * t);
			let parted = part_along_length(rest, root, t, part, 0.0);
			let lateral = (parted.x - rest.x).abs();
			assert!(
				lateral + 1e-5 >= previous,
				"lean must not shrink along the blade: t={t} lateral={lateral} prev={previous}"
			);
			previous = lateral;
		}
		assert!(previous > 0.2, "tip lean should be obvious, got {previous}");
		Ok(())
	}

	#[test]
	fn raster_length_bend_preserves_stem_length() -> anyhow::Result<()> {
		let root = Vec3::new(0.4, 0.0, 1.1);
		let part = Vec2::new(-0.7, 0.25);
		for t in [0.15_f32, 0.4, 0.7, 1.0] {
			let rest = root + Vec3::new(0.02 * t, 1.12 * t, 0.01 * t);
			let parted = part_along_length(rest, root, t, part, 0.05);
			let rest_len = rest.distance(root);
			let parted_len = parted.distance(root);
			assert!(
				(parted_len - rest_len).abs() < 1e-4,
				"t={t} rest_len={rest_len} parted_len={parted_len}"
			);
		}
		Ok(())
	}

	#[test]
	fn raster_length_bend_points_away_from_the_stamp_centre() -> anyhow::Result<()> {
		let root = Vec3::ZERO;
		let rest = Vec3::Y * 1.12;
		let left = part_along_length(rest, root, 1.0, Vec2::new(-0.6, 0.0), 0.0);
		let right = part_along_length(rest, root, 1.0, Vec2::new(0.6, 0.0), 0.0);
		assert!(left.x < rest.x, "left stamp should peel −X, got {left:?}");
		assert!(right.x > rest.x, "right stamp should peel +X, got {right:?}");
		assert!(left.y < rest.y && right.y < rest.y, "tips should lie down, not stretch");
		Ok(())
	}

	#[test]
	fn no_self_parting_when_occluder_is_in_front_or_coplanar() -> anyhow::Result<()> {
		assert_eq!(depth_gap_weight(4.0, 3.5, 1.6), 0.0);
		assert_eq!(depth_gap_weight(4.0, 4.0, 1.6), 0.0);
		assert_eq!(depth_gap_weight(4.0, 4.0005, 1.6), 0.0);
		Ok(())
	}

	#[test]
	fn falloff_peaks_just_behind_and_dies_with_distance() -> anyhow::Result<()> {
		let near = depth_gap_weight(5.0, 5.25, 1.6);
		let mid = depth_gap_weight(5.0, 5.7, 1.6);
		let far = depth_gap_weight(5.0, 7.0, 1.6);
		assert!(near > 0.5, "near {near}");
		assert!(mid > 0.15, "mid {mid}");
		assert!(far < 0.05, "far {far}");
		assert!(near > far);
		Ok(())
	}

	#[test]
	fn ground_is_rejected_character_is_kept() -> anyhow::Result<()> {
		let root_y = 0.0;
		assert!(ground_occluder_weight(0.02, root_y, 0.22) < 0.05);
		assert!(ground_occluder_weight(0.9, root_y, 0.22) > 0.95);
		Ok(())
	}

	#[test]
	fn world_field_parts_away_from_capsule() -> anyhow::Result<()> {
		let camera = Vec3::new(0.0, 0.6, 0.0);
		let forward = Vec3::Z;
		let right = Vec3::X;
		let capsule = PartingCapsule::character(Vec3::new(0.0, 0.9, 4.0));
		let left = capsule.sample_world(
			Vec3::new(-0.2, 0.0, 2.5),
			camera,
			forward,
			right,
			0.85,
			1.6,
			0.22,
		);
		let right_blade =
			capsule.sample_world(Vec3::new(0.2, 0.0, 2.5), camera, forward, right, 0.85, 1.6, 0.22);
		assert!(left.x < 0.0, "left blade should peel left, got {left:?}");
		assert!(right_blade.x > 0.0, "right blade should peel right, got {right_blade:?}");
		Ok(())
	}

	#[test]
	fn world_field_ignores_occluder_in_front_of_the_blade() -> anyhow::Result<()> {
		let camera = Vec3::new(0.0, 0.6, 0.0);
		let capsule = PartingCapsule::character(Vec3::new(0.0, 0.9, 1.0));
		let sample = capsule.sample_world(
			Vec3::new(0.15, 0.0, 3.0),
			camera,
			Vec3::Z,
			Vec3::X,
			0.85,
			1.6,
			0.22,
		);
		assert_eq!(sample, Vec2::ZERO);
		Ok(())
	}

	#[test]
	fn screen_gradient_points_away_from_occupancy() -> anyhow::Result<()> {
		let dir = screen_part_direction(Vec2::new(0.4, 0.0), 7);
		assert!(dir.x < 0.0);
		assert!((dir.length() - 1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn stronger_offset_is_deterministic_and_does_not_add() -> anyhow::Result<()> {
		let a = Vec2::new(0.4, 0.0);
		let b = Vec2::new(0.1, 0.1);
		assert_eq!(stronger_part_xz(a, b), a);
		assert_eq!(stronger_part_xz(b, a), a);
		assert_eq!(stronger_part_xz(a, b), stronger_part_xz(a, b));
		Ok(())
	}

	#[test]
	fn hash_fallback_is_deterministic() -> anyhow::Result<()> {
		let a = screen_part_direction(Vec2::ZERO, 42);
		let b = screen_part_direction(Vec2::ZERO, 42);
		assert_eq!(a, b);
		let c = screen_part_direction(Vec2::ZERO, 43);
		assert_eq!(a.length(), 1.0);
		assert_eq!(c.length(), 1.0);
		Ok(())
	}

	#[test]
	fn clip_roundtrip_matches_reconstruct() -> anyhow::Result<()> {
		let clip_from_world = Mat4::perspective_rh(1.0, 16.0 / 9.0, 0.1, 200.0)
			* Mat4::look_at_rh(Vec3::new(0.0, 1.0, 4.0), Vec3::ZERO, Vec3::Y);
		let world_from_clip = clip_from_world.inverse();
		let p = Vec3::new(0.4, 0.7, -1.2);
		let clip = clip_from_world * p.extend(1.0);
		let uv = clip_to_uv(clip);
		let depth = clip.z / clip.w.max(1e-5);
		let recovered = reconstruct_world(uv, depth, world_from_clip);
		assert!(recovered.distance(p) < 1e-3, "recovered {recovered:?} from {p:?} uv {uv:?}");
		Ok(())
	}
}
