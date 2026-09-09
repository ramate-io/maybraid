//! Look / aim samples the world bake ORs into Near.
//!
//! The camera cone is not a scene `LodRef`. Writers push [`IntelligenceFocusSample`]s
//! (firearm bore, lock point) without the bake knowing about firearm control.

use bevy::prelude::*;

/// Inset on each half-FOV so first-person 75° does not take the whole wedge.
pub const LOOK_FOV_INSET: f32 = 0.6;

/// Max XZ range for look / aim promotion. Combat may still go farther.
pub const LOOK_M: f32 = 150.0;

/// Elliptical view cone from a perspective camera, already inset.
#[derive(Clone, Copy, Debug)]
pub struct IntelligenceLook {
	pub origin: Vec3,
	pub forward: Vec3,
	pub right: Vec3,
	pub up: Vec3,
	pub half_fov_x: f32,
	pub half_fov_y: f32,
}

impl IntelligenceLook {
	/// Live [`Projection`] FOV and aspect, then [`LOOK_FOV_INSET`].
	pub fn from_perspective(
		transform: &GlobalTransform,
		perspective: &PerspectiveProjection,
	) -> Self {
		Self::from_perspective_inset(transform, perspective, LOOK_FOV_INSET)
	}

	pub fn from_perspective_inset(
		transform: &GlobalTransform,
		perspective: &PerspectiveProjection,
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
		}
	}

	pub fn contains(self, point: Vec3) -> bool {
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

/// True when the plant is in the inset camera cone or a focus sample, and inside
/// [`LOOK_M`].
pub fn look_promotes(
	dist_xz: f32,
	point: Vec3,
	look: Option<IntelligenceLook>,
	focus: &IntelligenceFocus,
) -> bool {
	dist_xz < LOOK_M && (look.is_some_and(|look| look.contains(point)) || focus.contains(point))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn looking_x(fov_y: f32, aspect: f32) -> IntelligenceLook {
		let tf =
			Transform::from_translation(Vec3::Y).looking_at(Vec3::new(20.0, 1.0, 0.0), Vec3::Y);
		IntelligenceLook::from_perspective(
			&GlobalTransform::from(tf),
			&PerspectiveProjection { fov: fov_y, aspect_ratio: aspect, ..default() },
		)
	}

	#[test]
	fn on_axis_mid_is_inside_inset_cone() {
		let look = looking_x(45_f32.to_radians(), 16.0 / 9.0);
		assert!(look.contains(Vec3::new(120.0, 1.0, 0.0)));
	}

	#[test]
	fn corner_of_wide_fov_is_outside_inset() {
		let look = looking_x(75_f32.to_radians(), 16.0 / 9.0);
		// 45° off-axis: inside a raw 75°×aspect wedge, outside 0.6 inset.
		assert!(!look.contains(Vec3::new(120.0, 1.0, 120.0)));
	}

	#[test]
	fn behind_camera_is_outside() {
		let look = looking_x(75_f32.to_radians(), 16.0 / 9.0);
		assert!(!look.contains(Vec3::new(-40.0, 1.0, 0.0)));
	}

	#[test]
	fn focus_sample_hits_a_tight_bore() {
		let sample = IntelligenceFocusSample {
			origin: Vec3::Y,
			dir: Vec3::X,
			max_range: LOOK_M,
			half_angle: 5_f32.to_radians(),
		};
		assert!(sample.contains(Vec3::new(80.0, 1.0, 0.0)));
		assert!(!sample.contains(Vec3::new(80.0, 1.0, 20.0)));
	}

	#[test]
	fn look_promotes_respects_range() {
		let look = looking_x(45_f32.to_radians(), 16.0 / 9.0);
		let focus = IntelligenceFocus::default();
		assert!(look_promotes(120.0, Vec3::new(120.0, 1.0, 0.0), Some(look), &focus));
		assert!(!look_promotes(180.0, Vec3::new(180.0, 1.0, 0.0), Some(look), &focus));
	}
}
