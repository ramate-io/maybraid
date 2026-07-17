//! Axis-aligned hyper-rectangles used as guillotine roots and leaf regions.

use bevy::math::{Vec2, Vec3, Vec4};

/// Axis-aligned hyper-rectangle `[min, max]` in `D` dimensions.
///
/// Component `i` spans the half-open intent `[min[i], max[i]]` with `max[i] >= min[i]`.
/// Naming follows shared procedural bounds style rather than generation-"cell" terminology.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds<const D: usize> {
	pub min: [f32; D],
	pub max: [f32; D],
}

pub type Bounds1 = Bounds<1>;
pub type Bounds2 = Bounds<2>;
pub type Bounds3 = Bounds<3>;
pub type Bounds4 = Bounds<4>;

impl<const D: usize> Bounds<D> {
	pub const fn new(min: [f32; D], max: [f32; D]) -> Self {
		Self { min, max }
	}

	pub fn from_origin_extent(origin: [f32; D], extent: [f32; D]) -> Self {
		let mut max = [0.0; D];
		for i in 0..D {
			max[i] = origin[i] + extent[i];
		}
		Self { min: origin, max }
	}

	/// Lower-left / component-wise minimum (RFC-127 seed anchor).
	pub const fn lower_left(&self) -> [f32; D] {
		self.min
	}

	pub fn extent(&self) -> [f32; D] {
		let mut e = [0.0; D];
		for i in 0..D {
			e[i] = self.max[i] - self.min[i];
		}
		e
	}

	pub fn volume(&self) -> f32 {
		let mut v = 1.0;
		for i in 0..D {
			v *= (self.max[i] - self.min[i]).max(0.0);
		}
		v
	}

	pub fn min_extent(&self) -> f32 {
		let e = self.extent();
		let mut m = f32::INFINITY;
		for i in 0..D {
			m = m.min(e[i]);
		}
		m
	}

	pub fn axis_span(&self, axis: usize) -> f32 {
		debug_assert!(axis < D);
		self.max[axis] - self.min[axis]
	}
}

impl Bounds<1> {
	pub fn from_interval(min: f32, max: f32) -> Self {
		Self::new([min], [max])
	}
}

impl Bounds<2> {
	pub fn from_vec2(min: Vec2, max: Vec2) -> Self {
		Self::new([min.x, min.y], [max.x, max.y])
	}

	pub fn as_vec2_min(&self) -> Vec2 {
		Vec2::new(self.min[0], self.min[1])
	}

	pub fn as_vec2_max(&self) -> Vec2 {
		Vec2::new(self.max[0], self.max[1])
	}
}

impl Bounds<3> {
	pub fn from_vec3(min: Vec3, max: Vec3) -> Self {
		Self::new([min.x, min.y, min.z], [max.x, max.y, max.z])
	}
}

impl Bounds<4> {
	pub fn from_vec4(min: Vec4, max: Vec4) -> Self {
		Self::new(
			[min.x, min.y, min.z, min.w],
			[max.x, max.y, max.z, max.w],
		)
	}
}
