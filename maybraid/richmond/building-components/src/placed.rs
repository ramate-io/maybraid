//! Geom-free placement and optional geometry+pose pairing.

use bevy_math::Vec3;

/// Translation / yaw / scale for a kit piece or continuous form in cell space.
///
/// Partition / floor / door kits are authored in a **normalized** local space
/// (angular arcs: radius \(1\), full height \(Y \in [0, 1]\); headers
/// \(Y \in [0, 0.2]\)). Buildings map that kit into cell space via [`Self::scale`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
	/// Translation in cell-local space.
	pub translation: Vec3,
	/// Yaw about +Y (radians).
	pub yaw: f32,
	/// Non-uniform scale applied to the normalized kit before yaw.
	///
	/// For a circular wall of radius \(R\) and storey height \(H\), use
	/// `Vec3::new(R, H, R)`.
	pub scale: Vec3,
}

impl Placement {
	pub const IDENTITY: Self = Self {
		translation: Vec3::ZERO,
		yaw: 0.0,
		scale: Vec3::ONE,
	};

	pub fn new(translation: Vec3, yaw: f32) -> Self {
		Self {
			translation,
			yaw,
			scale: Vec3::ONE,
		}
	}

	pub fn at_origin() -> Self {
		Self::IDENTITY
	}

	pub fn with_scale(mut self, scale: Vec3) -> Self {
		self.scale = scale;
		self
	}

	/// Compose a child placement under this parent (scale → yaw → translate).
	pub fn compose_child(self, child: Placement) -> Placement {
		Placement {
			translation: self.translation + rotate_yaw(child.translation * self.scale, self.yaw),
			yaw: self.yaw + child.yaw,
			scale: self.scale * child.scale,
		}
	}
}

impl Default for Placement {
	fn default() -> Self {
		Self::IDENTITY
	}
}

/// Geometry paired with a [`Placement`] (migration / internal tessellation aid).
#[derive(Debug, Clone, PartialEq)]
pub struct Placed<G> {
	pub geom: G,
	pub placement: Placement,
}

impl<G> Placed<G> {
	pub fn new(geom: G, translation: Vec3, yaw: f32) -> Self {
		Self {
			geom,
			placement: Placement::new(translation, yaw),
		}
	}

	pub fn at_origin(geom: G) -> Self {
		Self {
			geom,
			placement: Placement::at_origin(),
		}
	}

	pub fn with_placement(geom: G, placement: Placement) -> Self {
		Self { geom, placement }
	}

	pub fn with_scale(mut self, scale: Vec3) -> Self {
		self.placement = self.placement.with_scale(scale);
		self
	}

	pub fn map_geom<H>(self, f: impl FnOnce(G) -> H) -> Placed<H> {
		Placed {
			geom: f(self.geom),
			placement: self.placement,
		}
	}

	pub fn translation(&self) -> Vec3 {
		self.placement.translation
	}

	pub fn yaw(&self) -> f32 {
		self.placement.yaw
	}

	pub fn scale(&self) -> Vec3 {
		self.placement.scale
	}
}

pub(crate) fn rotate_yaw(v: Vec3, yaw: f32) -> Vec3 {
	let (s, c) = yaw.sin_cos();
	Vec3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
}
