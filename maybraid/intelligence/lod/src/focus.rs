//! Look / aim samples the world bake ORs into Near.
//!
//! Hip FOV is magnification 1: look range stays [`crate::IntelligenceBand::NEAR_M`].
//! Zoom narrows FOV; range is `NEAR_M * mag` inside the inset frustum.
//! Mailbox apply uses a wider inset and [`IntelligenceLook::in_frustum`].

use bevy::prelude::*;

use crate::IntelligenceBand;

/// Inset on each half-FOV so first-person 75° does not take the whole wedge.
pub const LOOK_FOV_INSET: f32 = 0.6;

/// Wider than [`LOOK_FOV_INSET`]: mailbox apply should cover the visible frame.
pub const LOOK_APPLY_FOV_INSET: f32 = 0.85;

/// Optical magnification of `current_fov` versus the hip / base FOV. Never below 1.
pub fn fov_magnification(base_fov: f32, current_fov: f32) -> f32 {
	let base = (base_fov.max(1e-3) * 0.5).tan();
	let current = (current_fov.max(1e-3) * 0.5).tan();
	(base / current).max(1.0)
}

/// Near radius under the current zoom. Hip (`mag == 1`) is [`IntelligenceBand::NEAR_M`].
pub fn look_near_m(base_fov: f32, current_fov: f32) -> f32 {
	IntelligenceBand::NEAR_M * fov_magnification(base_fov, current_fov)
}

/// Elliptical view cone from a perspective camera, already inset.
///
/// [`Self::near_m`] is hip Near scaled by FOV magnification.
#[derive(Clone, Copy, Debug)]
pub struct IntelligenceLook {
	pub origin: Vec3,
	pub forward: Vec3,
	pub right: Vec3,
	pub up: Vec3,
	pub half_fov_x: f32,
	pub half_fov_y: f32,
	pub near_m: f32,
}

impl IntelligenceLook {
	/// Live FOV versus `base_fov` (hip for the current POV), then [`LOOK_FOV_INSET`].
	pub fn from_perspective(
		transform: &GlobalTransform,
		perspective: &PerspectiveProjection,
		base_fov: f32,
	) -> Self {
		Self::from_perspective_inset(transform, perspective, base_fov, LOOK_FOV_INSET)
	}

	pub fn from_perspective_inset(
		transform: &GlobalTransform,
		perspective: &PerspectiveProjection,
		base_fov: f32,
		inset: f32,
	) -> Self {
		let inset = inset.clamp(0.05, 1.0);
		let half_y = perspective.fov.max(1e-3) * 0.5 * inset;
		let aspect = perspective.aspect_ratio.max(0.1);
		let half_x = ((perspective.fov * 0.5).tan() * aspect).atan() * inset;
		Self {
			origin: transform.translation(),
			forward: transform.forward().as_vec3(),
			right: transform.right().as_vec3(),
			up: transform.up().as_vec3(),
			half_fov_x: half_x,
			half_fov_y: half_y,
			near_m: look_near_m(base_fov, perspective.fov),
		}
	}

	/// Angular cone only. Combat Near past [`Self::near_m`] still counts if on-screen.
	pub fn in_frustum(self, point: Vec3) -> bool {
		let to = point - self.origin;
		let dist = to.length();
		if !dist.is_finite() || dist < 1e-3 {
			return false;
		}
		let dir = to / dist;
		let z = dir.dot(self.forward);
		if z <= 0.0 {
			return false;
		}
		let x = dir.dot(self.right);
		let y = dir.dot(self.up);
		x.abs() <= z * self.half_fov_x.tan() && y.abs() <= z * self.half_fov_y.tan()
	}

	pub fn contains(self, point: Vec3) -> bool {
		let dist = (point - self.origin).length();
		dist.is_finite() && dist <= self.near_m && self.in_frustum(point)
	}
}

/// Live apply-time cone. World republishes every Update from `LodViewer`.
///
/// `None` means no viewer this frame; mailbox rank-fills every Near body.
#[derive(Resource, Clone, Debug, Default)]
pub struct IntelligenceLookFrame {
	pub look: Option<IntelligenceLook>,
}

/// One aim / lock ray. Circular cone. Firearm and other writers push these.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntelligenceFocusSample {
	pub origin: Vec3,
	pub dir: Vec3,
	pub max_range: f32,
	pub half_angle: f32,
}

impl IntelligenceFocusSample {
	pub fn contains(self, point: Vec3) -> bool {
		let dir = self.dir.normalize_or_zero();
		if dir == Vec3::ZERO {
			return false;
		}
		let to = point - self.origin;
		let dist = to.length();
		if !dist.is_finite() || dist < 1e-3 || dist > self.max_range.max(0.0) {
			return false;
		}
		(to / dist).dot(dir) >= self.half_angle.max(0.0).cos()
	}
}

/// Bake-facing mailbox. Cap is small on purpose.
#[derive(Resource, Clone, Debug, Default)]
pub struct IntelligenceFocus {
	pub samples: Vec<IntelligenceFocusSample>,
}

impl IntelligenceFocus {
	pub const MAX_SAMPLES: usize = 4;

	pub fn clear(&mut self) {
		self.samples.clear();
	}

	pub fn push(&mut self, sample: IntelligenceFocusSample) {
		if self.samples.len() < Self::MAX_SAMPLES {
			self.samples.push(sample);
		}
	}

	pub fn contains(&self, point: Vec3) -> bool {
		self.samples.iter().any(|sample| sample.contains(point))
	}
}

/// True when the plant is in the inset camera cone or a focus sample.
pub fn look_promotes(
	point: Vec3,
	look: Option<IntelligenceLook>,
	focus: &IntelligenceFocus,
) -> bool {
	look.is_some_and(|look| look.contains(point)) || focus.contains(point)
}

/// True when mailbox apply should treat this plant as on-screen.
///
/// Uses [`IntelligenceLook::in_frustum`] (no `near_m` cap) plus focus samples.
pub fn look_applies(
	point: Vec3,
	look: Option<IntelligenceLook>,
	focus: &IntelligenceFocus,
) -> bool {
	look.is_some_and(|look| look.in_frustum(point)) || focus.contains(point)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn looking_x(current_fov: f32, base_fov: f32) -> IntelligenceLook {
		let tf =
			Transform::from_translation(Vec3::Y).looking_at(Vec3::new(20.0, 1.0, 0.0), Vec3::Y);
		IntelligenceLook::from_perspective(
			&GlobalTransform::from(tf),
			&PerspectiveProjection { fov: current_fov, aspect_ratio: 16.0 / 9.0, ..default() },
			base_fov,
		)
	}

	#[test]
	fn hip_zoom_does_not_extend_near() {
		let look = looking_x(75_f32.to_radians(), 75_f32.to_radians());
		assert!((look.near_m - IntelligenceBand::NEAR_M).abs() < 1e-3);
		assert!(!look.contains(Vec3::new(120.0, 1.0, 0.0)));
		assert!(look.contains(Vec3::new(40.0, 1.0, 0.0)));
	}

	#[test]
	fn five_x_extends_near_inside_the_frustum() {
		let base = 75_f32.to_radians();
		let current = 15_f32.to_radians();
		let look = looking_x(current, base);
		assert!(look.near_m > 180.0);
		assert!(look.contains(Vec3::new(180.0, 1.0, 0.0)));
		assert!(!look.contains(Vec3::new(180.0, 1.0, 80.0)));
	}

	#[test]
	fn corner_of_wide_fov_is_outside_inset() {
		let look = looking_x(75_f32.to_radians(), 75_f32.to_radians());
		assert!(!look.contains(Vec3::new(40.0, 1.0, 40.0)));
	}

	#[test]
	fn behind_camera_is_outside() {
		let look = looking_x(75_f32.to_radians(), 75_f32.to_radians());
		assert!(!look.contains(Vec3::new(-40.0, 1.0, 0.0)));
	}

	#[test]
	fn focus_sample_hits_a_tight_bore() {
		let sample = IntelligenceFocusSample {
			origin: Vec3::Y,
			dir: Vec3::X,
			max_range: 400.0,
			half_angle: 5_f32.to_radians(),
		};
		assert!(sample.contains(Vec3::new(80.0, 1.0, 0.0)));
		assert!(!sample.contains(Vec3::new(80.0, 1.0, 20.0)));
	}

	#[test]
	fn magnification_is_at_least_one() {
		assert!((fov_magnification(75_f32.to_radians(), 75_f32.to_radians()) - 1.0).abs() < 1e-4);
		assert!(fov_magnification(75_f32.to_radians(), 90_f32.to_radians()) >= 1.0);
		assert!(fov_magnification(75_f32.to_radians(), 15_f32.to_radians()) > 5.0);
	}

	#[test]
	fn apply_inset_covers_a_promote_miss() {
		let tf =
			Transform::from_translation(Vec3::Y).looking_at(Vec3::new(20.0, 1.0, 0.0), Vec3::Y);
		let perspective = PerspectiveProjection {
			fov: 75_f32.to_radians(),
			aspect_ratio: 16.0 / 9.0,
			..default()
		};
		let promote = IntelligenceLook::from_perspective(
			&GlobalTransform::from(tf),
			&perspective,
			75_f32.to_radians(),
		);
		let apply = IntelligenceLook::from_perspective_inset(
			&GlobalTransform::from(tf),
			&perspective,
			75_f32.to_radians(),
			LOOK_APPLY_FOV_INSET,
		);
		let edge = Vec3::new(40.0, 1.0, 40.0);
		assert!(!promote.contains(edge));
		assert!(apply.in_frustum(edge));
	}

	#[test]
	fn frustum_keeps_distant_on_axis_combat() {
		let look = looking_x(75_f32.to_radians(), 75_f32.to_radians());
		let far = Vec3::new(300.0, 1.0, 0.0);
		assert!(!look.contains(far));
		assert!(look.in_frustum(far));
		assert!(look_applies(far, Some(look), &IntelligenceFocus::default()));
	}
}
