//! Two-rail strip of best-fit rectangles with optional per-bay clips.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::panel_complex::{PanelComplex, PanelComplexJointPolicy, PanelPoint};
use crate::paneling::rect_fit::fit_rectangle_corners;
use crate::paneling::rectangle::ClippedRectangle;

/// One flushed region of a [`ClippedRectangularStrip`].
#[derive(Debug, Clone, PartialEq)]
pub enum ClippedRectangularStripPiece {
	Solid(PanelComplex),
	Clipped(ClippedRectangle),
}

impl ClippedRectangularStripPiece {
	pub fn as_complex(&self) -> &PanelComplex {
		match self {
			Self::Solid(c) => c,
			Self::Clipped(r) => r.as_complex(),
		}
	}
}

/// Two-rail rectangular strip with optional per-bay clips.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedRectangularStrip {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	authored: Vec<(PanelPoint, PanelPoint)>,
	pieces: Vec<ClippedRectangularStripPiece>,
	open_solid: Option<PanelComplex>,
}

impl ClippedRectangularStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			authored: Vec::new(),
			pieces: Vec::new(),
			open_solid: None,
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		if let Some(c) = self.open_solid.as_mut() {
			c.set_joint_policy(joint_policy);
		}
		for piece in &mut self.pieces {
			match piece {
				ClippedRectangularStripPiece::Solid(c) => {
					c.set_joint_policy(joint_policy);
				}
				ClippedRectangularStripPiece::Clipped(r) => {
					r.set_joint_policy(joint_policy);
				}
			}
		}
		self
	}

	pub fn from_lines(
		style: PanelStyle,
		rail_a: impl IntoIterator<Item = impl Into<PanelPoint>>,
		rail_b: impl IntoIterator<Item = impl Into<PanelPoint>>,
		clips: impl IntoIterator<Item = Option<Vec<Vec3>>>,
	) -> Self {
		let rail_a: Vec<PanelPoint> = rail_a.into_iter().map(Into::into).collect();
		let rail_b: Vec<PanelPoint> = rail_b.into_iter().map(Into::into).collect();
		let mut clips: Vec<Option<Vec<Vec3>>> = clips.into_iter().collect();
		if rail_a.len() != rail_b.len() || rail_a.len() < 2 {
			debug_assert!(false, "ClippedRectangularStrip::from_lines bad rail lengths");
			return Self::new(style);
		}
		let bay_count = rail_a.len() - 1;
		if clips.len() != bay_count {
			debug_assert!(false, "ClippedRectangularStrip::from_lines clips/bay mismatch");
			clips.resize_with(bay_count, || None);
		}
		let mut strip = Self::new(style);
		strip.add_pair(rail_a[0], rail_b[0], None::<Vec<Vec3>>);
		for i in 0..bay_count {
			strip.add_pair(rail_a[i + 1], rail_b[i + 1], clips[i].clone());
		}
		strip.finish();
		strip
	}

	pub fn add_pair(
		&mut self,
		rail_a: impl Into<PanelPoint>,
		rail_b: impl Into<PanelPoint>,
		clip: Option<impl IntoIterator<Item = impl Into<Vec3>>>,
	) -> &mut Self {
		let rail_a = rail_a.into();
		let rail_b = rail_b.into();
		let clip: Option<Vec<Vec3>> = clip.map(|c| c.into_iter().map(Into::into).collect());

		if self.authored.is_empty() {
			self.authored.push((rail_a, rail_b));
			return self;
		}
		let (prev_a, prev_b) = *self.authored.last().unwrap();
		self.authored.push((rail_a, rail_b));

		match clip {
			None => self.append_solid_bay(prev_a, rail_a, prev_b, rail_b),
			Some(c) => {
				self.flush_open_solid();
				let r = ClippedRectangle::new(self.style, prev_a, rail_a, prev_b, rail_b, c)
					.with_joint_policy(self.joint_policy);
				self.pieces
					.push(ClippedRectangularStripPiece::Clipped(r));
				self
			}
		}
	}

	pub fn finish(&mut self) -> &mut Self {
		self.flush_open_solid();
		self
	}

	pub fn pieces(&self) -> &[ClippedRectangularStripPiece] {
		&self.pieces
	}

	pub fn authored_stations(&self) -> &[(PanelPoint, PanelPoint)] {
		&self.authored
	}

	fn append_solid_bay(
		&mut self,
		prev_a: PanelPoint,
		curr_a: PanelPoint,
		prev_b: PanelPoint,
		curr_b: PanelPoint,
	) -> &mut Self {
		let Some([fa0, fa1, fb0, fb1]) =
			fit_rectangle_corners(prev_a.position, curr_a.position, prev_b.position, curr_b.position)
		else {
			debug_assert!(false, "ClippedRectangularStrip: degenerate bay fit");
			return self;
		};
		if self.open_solid.is_none() {
			self.open_solid = Some(PanelComplex::new(self.style).with_joint_policy(self.joint_policy));
		}
		let complex = self.open_solid.as_mut().unwrap();
		let id0 = complex.insert_point_thick(fa0, prev_a.thickness);
		let id1 = complex.insert_point_thick(fa1, curr_a.thickness);
		let id2 = complex.insert_point_thick(fb0, prev_b.thickness);
		let id3 = complex.insert_point_thick(fb1, curr_b.thickness);
		complex.add_quad(id0, id1, id2, id3);
		self
	}

	fn flush_open_solid(&mut self) {
		if let Some(c) = self.open_solid.take() {
			if !c.triangles().is_empty() {
				self.pieces.push(ClippedRectangularStripPiece::Solid(c));
			}
		}
	}
}

impl BuildingComponents for ClippedRectangularStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			out.extend(piece.as_complex().panel_nodes_for_level(level));
		}
		if let Some(c) = &self.open_solid {
			out.extend(c.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			out.extend(piece.as_complex().joint_nodes_for_level(level));
		}
		if let Some(c) = &self.open_solid {
			out.extend(c.joint_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn middle_clip_splits_pieces() {
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
		let clip = vec![
			Vec3::new(0.5, 0.0, 0.5),
			Vec3::new(1.5, 0.0, 0.5),
			Vec3::new(1.5, 0.0, 1.5),
			Vec3::new(0.5, 0.0, 1.5),
		];
		let s = ClippedRectangularStrip::from_lines(
			PanelStyle::RoughStonework,
			a,
			b,
			[Some(clip), None],
		);
		assert_eq!(s.pieces().len(), 2);
		assert!(matches!(
			s.pieces()[0],
			ClippedRectangularStripPiece::Clipped(_)
		));
		assert!(matches!(s.pieces()[1], ClippedRectangularStripPiece::Solid(_)));
	}
}
