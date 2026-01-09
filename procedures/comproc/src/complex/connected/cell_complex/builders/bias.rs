use crate::complex::connected::cell_complex::{
	CellComplex3d, CellComplex3dBuilder, Edge, Face, Vertex,
};
use bevy::prelude::*;
use std::marker::PhantomData;

/// The bias is a helper designed to render a bias value
/// based on the scalar projection of a position onto a given ray.
pub struct RayBias {
	start: Vec3,
	bias_vector: Vec3,
}

impl RayBias {
	pub fn new(start: Vec3, bias_vector: Vec3) -> Self {
		Self { start, bias_vector }
	}

	/// The scalar projection is the projection of the position onto the bias vector,
	/// normalized by the length of the bias vector.
	fn scalar_projection(&self, position: Vec3) -> f32 {
		let delta = position - self.start;
		delta.dot(self.bias_vector) / self.bias_vector.length_squared()
	}

	/// Computes the bias.
	///
	/// Generally, 0 to 1 is increasingly biased towards some behavior,
	/// while outside the unit interval is none.
	pub fn bias(&self, position: Vec3) -> f32 {
		self.scalar_projection(position)
	}
}

pub trait BiasCellComplex3dBuilder<V: Vertex, E: Edge<V>, F: Face<V, E>> {
	fn next_bias_ray_face(
		&mut self,
		complex: &CellComplex3d<V, E, F>,
		last_face: Option<F>,
		bias_ray: &RayBias,
	) -> Option<F>;
}

pub struct RayBiasBuilder<
	V: Vertex,
	E: Edge<V>,
	F: Face<V, E>,
	T: BiasCellComplex3dBuilder<V, E, F>,
> {
	bias_ray: RayBias,
	builder: T,
	__marker: PhantomData<(V, E, F)>,
}

impl<V: Vertex, E: Edge<V>, F: Face<V, E>, T: BiasCellComplex3dBuilder<V, E, F>>
	CellComplex3dBuilder<V, E, F> for RayBiasBuilder<V, E, F, T>
{
	fn next_face(&mut self, complex: &CellComplex3d<V, E, F>, last_face: Option<F>) -> Option<F> {
		self.builder.next_bias_ray_face(complex, last_face, &self.bias_ray)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_scalar_projection() {
		let bias = RayBias::new(Vec3::ZERO, Vec3::Y);
		assert_eq!(bias.scalar_projection(Vec3::new(0.0, 1.0, 0.0)), 1.0);
		assert_eq!(bias.scalar_projection(Vec3::new(0.0, 1.0, 1.0)), 1.0);
		assert_eq!(bias.scalar_projection(Vec3::new(1.0, 1.0, -1.0)), 1.0);
		assert_eq!(bias.scalar_projection(Vec3::new(1.0, -1.0, 1.0)), -1.0);

		let y_eq_1x_bias = RayBias::new(Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0));
		assert_eq!(y_eq_1x_bias.scalar_projection(Vec3::new(1.0, 1.0, 0.0)), 1.0);
		assert_eq!(y_eq_1x_bias.scalar_projection(Vec3::new(1.0, 0.5, 1.0)), 0.75);
	}
}
