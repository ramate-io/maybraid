pub mod analysis;
pub mod capsule;
pub mod combinators;
pub mod ellipsoid;
pub mod sphere;
pub mod tetradhedron;
pub mod trapezoidal_prism;
pub mod tube;

pub use analysis::bounds::Bounds;
pub use analysis::interval::{Sign, SignBoundary, SignUniformInterval, SignUniformIntervals};
pub use capsule::CapsuleSdf;
pub use combinators::{
	AddY, Difference, Elongate, Intersection, RotateAlongRay, RotateY, Round, Scale,
	SmoothDifference, SmoothIntersection, SmoothUnion, Translate, Union,
};
pub use ellipsoid::EllipsoidSdf;
pub use sphere::SphereSdf;
pub use tube::{Ellipse3d, TubeSdf};

use bevy::prelude::*;

/// Trait for Signed Distance Fields
/// Returns the signed distance from a point to the surface:
/// - Negative: inside/below the surface
/// - Zero: on the surface
/// - Positive: outside/above the surface
pub trait Sdf: Send + Sync {
	fn distance(&self, p: Vec3) -> f32;

	/// Computes intervals along Y of sign uniformity for a given (x, z) position.
	///
	/// This is useful for voxel grid optimizations as you can skip ahead to the next
	/// new Y value that need be sampled.
	///
	/// You could do this over any plane. But, giving x and z applies nicely to the plain in which
	/// most gameplay is defined.
	fn sign_uniform_on_y(&self, _x: f32, _z: f32) -> SignUniformIntervals {
		SignUniformIntervals::default()
	}

	/// Returns the bounds of the SDF, i.e., the region over which the SDF is defined.
	/// This can form pessimistic boundaries for analysis of the SDF.
	///
	/// For example, when unioning two SDFs, you can say everything within the bounds of either SDF
	/// is Top. Everything outside the intesecting bounds can take the value of which SDF is defined.
	///
	/// Often times, you can compute tighter bounds. But, this is useful when doing so is computationally expensive.
	fn bounds(&self) -> Bounds {
		Bounds::Unbounded
	}

	/// The stateful translation of the SDF.
	fn translation(&self) -> Vec3 {
		Vec3::ZERO
	}

	/// The stateful rotation of the SDF.
	fn rotation(&self) -> Quat {
		Quat::IDENTITY
	}

	/// The stateful scale of the SDF.
	fn scale(&self) -> Vec3 {
		Vec3::ONE
	}
}

impl<T: Sdf + ?Sized> Sdf for std::sync::Arc<T> {
	fn distance(&self, p: Vec3) -> f32 {
		self.as_ref().distance(p)
	}

	fn sign_uniform_on_y(&self, x: f32, z: f32) -> SignUniformIntervals {
		self.as_ref().sign_uniform_on_y(x, z)
	}

	fn bounds(&self) -> Bounds {
		self.as_ref().bounds()
	}

	fn translation(&self) -> Vec3 {
		self.as_ref().translation()
	}

	fn rotation(&self) -> Quat {
		self.as_ref().rotation()
	}

	fn scale(&self) -> Vec3 {
		self.as_ref().scale()
	}
}
