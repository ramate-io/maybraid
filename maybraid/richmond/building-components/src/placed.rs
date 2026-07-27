//! Placed continuous geometry and tessellation into geometry components.

use bevy_math::Vec3;

/// Geometry plus a local pose used when placing a feature in a cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Placed<G> {
	pub geom: G,
	/// Translation in cell-local space.
	pub translation: Vec3,
	/// Yaw about +Y (radians).
	pub yaw: f32,
}

impl<G> Placed<G> {
	pub fn new(geom: G, translation: Vec3, yaw: f32) -> Self {
		Self {
			geom,
			translation,
			yaw,
		}
	}

	pub fn at_origin(geom: G) -> Self {
		Self::new(geom, Vec3::ZERO, 0.0)
	}

	pub fn map_geom<H>(self, f: impl FnOnce(G) -> H) -> Placed<H> {
		Placed {
			geom: f(self.geom),
			translation: self.translation,
			yaw: self.yaw,
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
				translation: self.translation + rotate_yaw(child.translation, self.yaw),
				yaw: self.yaw + child.yaw,
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
