//! Geom-free placement for vegetation kits.

use bevy_math::{EulerRot, Quat, Vec3};

/// Translation / yaw / pitch / roll / scale for a kit piece in tree-local space.
///
/// Stick kits are authored vertically: \(Y \in [0, 1]\), \(X = Z \in [-0.2, 0.2]\).
/// Placement maps that kit into tree space (base at segment start, \(+Y\) along the
/// segment). Rotation order is intrinsic [`EulerRot::YXZ`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
	pub translation: Vec3,
	pub yaw: f32,
	pub pitch: f32,
	pub roll: f32,
	pub scale: Vec3,
}

impl Placement {
	pub const IDENTITY: Self =
		Self { translation: Vec3::ZERO, yaw: 0.0, pitch: 0.0, roll: 0.0, scale: Vec3::ONE };

	pub fn new(translation: Vec3, yaw: f32) -> Self {
		Self { translation, yaw, pitch: 0.0, roll: 0.0, scale: Vec3::ONE }
	}

	pub fn at_origin() -> Self {
		Self::IDENTITY
	}

	pub fn with_pitch(mut self, pitch: f32) -> Self {
		self.pitch = pitch;
		self
	}

	pub fn with_roll(mut self, roll: f32) -> Self {
		self.roll = roll;
		self
	}

	pub fn with_scale(mut self, scale: Vec3) -> Self {
		self.scale = scale;
		self
	}

	pub fn with_rotation(mut self, rotation: Quat) -> Self {
		let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
		self.yaw = yaw;
		self.pitch = pitch;
		self.roll = roll;
		self
	}

	pub fn rotation(self) -> Quat {
		Quat::from_euler(EulerRot::YXZ, self.yaw, self.pitch, self.roll)
	}

	/// Stick segment: base at `start`, \(+Y\) along `dir`, length / girth from kit space.
	///
	/// Kit half-extent on \(X/Z\) is [`crate::sticks::STICK_KIT_HALF`]; world radius `radius`
	/// maps to scale \(=\texttt{radius} / \texttt{STICK\_KIT\_HALF}\).
	pub fn stick_segment(start: Vec3, dir: Vec3, length: f32, radius: f32) -> Option<Self> {
		let len_sq = dir.length_squared();
		if len_sq < 1e-12 || length < 1e-6 {
			return None;
		}
		let d = dir / len_sq.sqrt();
		let rotation = Quat::from_rotation_arc(Vec3::Y, d);
		let girth = (radius / crate::sticks::STICK_KIT_HALF).max(1e-4);
		Some(
			Self::new(start, 0.0)
				.with_rotation(rotation)
				.with_scale(Vec3::new(girth, length, girth)),
		)
	}

	/// Uniform foliage ball / splay at `center` with world radius `radius`.
	pub fn foliage_uniform(center: Vec3, radius: f32) -> Self {
		Self::new(center, 0.0).with_scale(Vec3::splat(radius.max(1e-4)))
	}

	/// Straight frond segment: base at `start`, \(+Y\) along `dir`, square girth from `width`.
	///
	/// Kit half-extent on \(X/Z\) is [`crate::FROND_SEGMENT_KIT_HALF`]; world full width
	/// `width` maps to scale \(=\texttt{width} / (2 \cdot \texttt{FROND\_SEGMENT\_KIT\_HALF})\).
	pub fn frond_segment(start: Vec3, dir: Vec3, length: f32, width: f32) -> Option<Self> {
		let len_sq = dir.length_squared();
		if len_sq < 1e-12 || length < 1e-6 {
			return None;
		}
		let d = dir / len_sq.sqrt();
		let rotation = Quat::from_rotation_arc(Vec3::Y, d);
		let girth = (width * 0.5 / crate::FROND_SEGMENT_KIT_HALF).max(1e-4);
		Some(
			Self::new(start, 0.0)
				.with_rotation(rotation)
				.with_scale(Vec3::new(girth, length, girth)),
		)
	}
}

impl Default for Placement {
	fn default() -> Self {
		Self::IDENTITY
	}
}
