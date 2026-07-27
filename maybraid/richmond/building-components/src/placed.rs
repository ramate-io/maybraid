//! Placed continuous geometry and tessellation into geometry components.

use bevy_math::Vec3;

/// Geometry plus a local pose used when placing a feature in a cell.
///
/// Partition / floor / door kit pieces are authored in a **normalized** local
/// space (angular arcs: radius \(1\), full height \(Y \in [0, 1]\)). Buildings
/// map that kit into cell space via [`Self::scale`].
#[derive(Debug, Clone, PartialEq)]
pub struct Placed<G> {
	pub geom: G,
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

impl<G> Placed<G> {
	pub fn new(geom: G, translation: Vec3, yaw: f32) -> Self {
		Self {
			geom,
			translation,
			yaw,
			scale: Vec3::ONE,
		}
	}

	pub fn at_origin(geom: G) -> Self {
		Self::new(geom, Vec3::ZERO, 0.0)
	}

	pub fn with_scale(mut self, scale: Vec3) -> Self {
		self.scale = scale;
		self
	}

	pub fn map_geom<H>(self, f: impl FnOnce(G) -> H) -> Placed<H> {
		Placed {
			geom: f(self.geom),
			translation: self.translation,
			yaw: self.yaw,
			scale: self.scale,
		}
	}

	/// Tessellate this placed continuous form; child poses are composed with `self`.
	pub fn into_geometry_components(&self) -> Vec<Placed<G::Component>>
	where
		G: IntoGeometryComponents,
	{
		self.geom
			.into_geometry_components()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				translation: self.translation
					+ rotate_yaw(child.translation * self.scale, self.yaw),
				yaw: self.yaw + child.yaw,
				scale: self.scale * child.scale,
			})
			.collect()
	}
}

/// Tessellate continuous geometry into placed normalized kit pieces.
pub trait IntoGeometryComponents {
	type Component;

	fn into_geometry_components(&self) -> Vec<Placed<Self::Component>>;
}

fn rotate_yaw(v: Vec3, yaw: f32) -> Vec3 {
	let (s, c) = yaw.sin_cos();
	Vec3::new(c * v.x + s * v.z, v.y, -s * v.x + c * v.z)
}
