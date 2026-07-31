//! Two-rail strip of best-fit ordinary rectangles.

use richmond_building_components::panels::PanelStyle;

use crate::paneling::panel_complex::{PanelComplex, PanelComplexJointPolicy, PanelPoint};
use crate::paneling::rect_fit::fit_rectangle_corners;

/// Equal-station strip; each bay is an independently fitted rectangle on one [`PanelComplex`].
///
/// Adjacent bays do not share point ids (each bay’s rectangle is re-fit), so crease joints
/// appear only within each bay’s diagonal unless fits happen to coincide.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularStrip {
	complex: PanelComplex,
	/// Authored (pre-fit) stations for query.
	authored: Vec<(PanelPoint, PanelPoint)>,
}

impl RectangularStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			complex: PanelComplex::new(style),
			authored: Vec::new(),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	pub fn from_lines(
		style: PanelStyle,
		rail_a: impl IntoIterator<Item = impl Into<PanelPoint>>,
		rail_b: impl IntoIterator<Item = impl Into<PanelPoint>>,
	) -> Self {
		let rail_a: Vec<PanelPoint> = rail_a.into_iter().map(Into::into).collect();
		let rail_b: Vec<PanelPoint> = rail_b.into_iter().map(Into::into).collect();
		if rail_a.len() != rail_b.len() || rail_a.len() < 2 {
			debug_assert!(
				false,
				"RectangularStrip::from_lines requires equal lengths >= 2"
			);
			return Self::new(style);
		}
		let mut strip = Self::new(style);
		for (a, b) in rail_a.into_iter().zip(rail_b) {
			strip.add_pair(a, b);
		}
		strip
	}

	pub fn add_pair(
		&mut self,
		rail_a: impl Into<PanelPoint>,
		rail_b: impl Into<PanelPoint>,
	) -> &mut Self {
		let rail_a = rail_a.into();
		let rail_b = rail_b.into();
		self.authored.push((rail_a, rail_b));
		if self.authored.len() == 1 {
			return self;
		}
		let (prev_a, prev_b) = self.authored[self.authored.len() - 2];
		let Some([fa0, fa1, fb0, fb1]) =
			fit_rectangle_corners(prev_a.position, rail_a.position, prev_b.position, rail_b.position)
		else {
			debug_assert!(false, "RectangularStrip: degenerate bay fit");
			return self;
		};
		let id0 = self.complex.insert_point_thick(fa0, prev_a.thickness);
		let id1 = self.complex.insert_point_thick(fa1, rail_a.thickness);
		let id2 = self.complex.insert_point_thick(fb0, prev_b.thickness);
		let id3 = self.complex.insert_point_thick(fb1, rail_b.thickness);
		self.complex.add_quad(id0, id1, id2, id3);
		self
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

	pub fn authored_stations(&self) -> &[(PanelPoint, PanelPoint)] {
		&self.authored
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;

	#[test]
	fn three_stations_two_quads() {
		let a = [
			Vec3::ZERO,
			Vec3::new(0.0, 0.0, 2.0),
			Vec3::new(0.0, 0.0, 4.0),
		];
		let b = [
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 2.0),
			Vec3::new(2.0, 0.0, 4.0),
		];
		let s = RectangularStrip::from_lines(PanelStyle::RoughStonework, a, b);
		assert_eq!(s.as_complex().triangles().len(), 4);
		assert_eq!(s.authored_stations().len(), 3);
	}
}
