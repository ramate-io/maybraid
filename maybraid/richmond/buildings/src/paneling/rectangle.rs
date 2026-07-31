//! Best-fit ordinary rectangle (and clipped variant) for a ruled bay.

use bevy_math::Vec3;
use richmond_building_components::panels::PanelStyle;

use crate::paneling::clipped_quad_panel::ClippedQuadPanel;
use crate::paneling::panel_complex::{PanelComplex, PanelComplexJointPolicy, PanelPoint};
use crate::paneling::quad_panel::QuadPanel;
use crate::paneling::rect_fit::fit_rectangle_corners;

/// Solid best-fit rectangle for authored bay corners `{a0,a1,b0,b1}`.
#[derive(Debug, Clone, PartialEq)]
pub struct Rectangle {
	pub style: PanelStyle,
	/// Authored (possibly skew) corners.
	pub a0: PanelPoint,
	pub a1: PanelPoint,
	pub b0: PanelPoint,
	pub b1: PanelPoint,
	/// Fitted ordinary-rectangle corners used for the mesh.
	pub fitted: [PanelPoint; 4],
	complex: PanelComplex,
}

impl Rectangle {
	pub fn new(
		style: PanelStyle,
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
	) -> Self {
		let a0 = a0.into();
		let a1 = a1.into();
		let b0 = b0.into();
		let b1 = b1.into();
		let fitted = fit_points(a0, a1, b0, b1);
		let complex = QuadPanel::new(style, fitted[0], fitted[1], fitted[2], fitted[3]).into_complex();
		Self {
			style,
			a0,
			a1,
			b0,
			b1,
			fitted,
			complex,
		}
	}

	pub fn rough_stone(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a0, a1, b0, b1)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.complex = self.complex.with_joint_policy(joint_policy);
		self
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.complex
	}

	pub fn into_complex(self) -> PanelComplex {
		self.complex
	}
}

/// Best-fit rectangle with a closed world clip on the fitted panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedRectangle {
	pub style: PanelStyle,
	pub a0: PanelPoint,
	pub a1: PanelPoint,
	pub b0: PanelPoint,
	pub b1: PanelPoint,
	pub fitted: [PanelPoint; 4],
	pub clip: Vec<Vec3>,
	inner: ClippedQuadPanel,
}

impl ClippedRectangle {
	pub fn new(
		style: PanelStyle,
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		let a0 = a0.into();
		let a1 = a1.into();
		let b0 = b0.into();
		let b1 = b1.into();
		let clip: Vec<Vec3> = clip.into_iter().map(Into::into).collect();
		let fitted = fit_points(a0, a1, b0, b1);
		let inner = ClippedQuadPanel::new(
			style,
			fitted[0],
			fitted[1],
			fitted[2],
			fitted[3],
			clip.iter().copied(),
		);
		Self {
			style,
			a0,
			a1,
			b0,
			b1,
			fitted,
			clip,
			inner,
		}
	}

	pub fn rough_stone(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::RoughStonework, a0, a1, b0, b1, clip)
	}

	pub fn shepherds_thatch(
		a0: impl Into<PanelPoint>,
		a1: impl Into<PanelPoint>,
		b0: impl Into<PanelPoint>,
		b1: impl Into<PanelPoint>,
		clip: impl IntoIterator<Item = impl Into<Vec3>>,
	) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a0, a1, b0, b1, clip)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.inner = self.inner.with_joint_policy(joint_policy);
		self
	}

	pub fn set_joint_policy(&mut self, joint_policy: PanelComplexJointPolicy) -> &mut Self {
		self.inner.set_joint_policy(joint_policy);
		self
	}

	pub fn as_complex(&self) -> &PanelComplex {
		self.inner.as_complex()
	}

	pub fn into_complex(self) -> PanelComplex {
		self.inner.into_complex()
	}
}

fn fit_points(a0: PanelPoint, a1: PanelPoint, b0: PanelPoint, b1: PanelPoint) -> [PanelPoint; 4] {
	match fit_rectangle_corners(a0.position, a1.position, b0.position, b1.position) {
		Some([fa0, fa1, fb0, fb1]) => [
			PanelPoint::new(fa0, a0.thickness),
			PanelPoint::new(fa1, a1.thickness),
			PanelPoint::new(fb0, b0.thickness),
			PanelPoint::new(fb1, b1.thickness),
		],
		None => [a0, a1, b0, b1],
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solid_has_two_tris() {
		let r = Rectangle::rough_stone(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 1.0),
			Vec3::new(2.0, 0.0, 1.0),
		);
		assert_eq!(r.as_complex().triangles().len(), 2);
	}

	#[test]
	fn clipped_leaves_hole() {
		let clip = [
			Vec3::new(0.5, 0.0, 0.3),
			Vec3::new(1.5, 0.0, 0.3),
			Vec3::new(1.5, 0.0, 0.7),
			Vec3::new(0.5, 0.0, 0.7),
		];
		let r = ClippedRectangle::rough_stone(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 1.0),
			Vec3::new(2.0, 0.0, 1.0),
			clip,
		);
		assert!(r.as_complex().triangles().len() >= 3);
	}
}
