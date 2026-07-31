//! Two-rail strip of best-fit rectangles with optional per-bay inset openings.

use lod::gen::LodSceneLevel;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::panel_complex::PanelPoint;
use crate::paneling::rect_fit::RectInset;
use crate::paneling::rectangle::{ClippedRectangle, Rectangle};

/// One bay of a [`ClippedRectangularStrip`].
#[derive(Debug, Clone, PartialEq)]
pub enum ClippedRectangularStripPiece {
	Solid(Rectangle),
	Clipped(ClippedRectangle),
}

impl ClippedRectangularStripPiece {
	pub fn panels(&self) -> Vec<&PanelNode> {
		match self {
			Self::Solid(r) => vec![r.panel_node()],
			Self::Clipped(r) => r.panels().iter().collect(),
		}
	}
}

/// Two-rail rectangular strip with optional per-bay [`RectInset`] openings.
#[derive(Debug, Clone, PartialEq)]
pub struct ClippedRectangularStrip {
	style: PanelStyle,
	authored: Vec<(PanelPoint, PanelPoint)>,
	pieces: Vec<ClippedRectangularStripPiece>,
}

impl ClippedRectangularStrip {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			authored: Vec::new(),
			pieces: Vec::new(),
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
		insets: impl IntoIterator<Item = Option<RectInset>>,
	) -> Self {
		let rail_a: Vec<PanelPoint> = rail_a.into_iter().map(Into::into).collect();
		let rail_b: Vec<PanelPoint> = rail_b.into_iter().map(Into::into).collect();
		let mut insets: Vec<Option<RectInset>> = insets.into_iter().collect();
		if rail_a.len() != rail_b.len() || rail_a.len() < 2 {
			debug_assert!(false, "ClippedRectangularStrip::from_lines bad rail lengths");
			return Self::new(style);
		}
		let bay_count = rail_a.len() - 1;
		if insets.len() != bay_count {
			debug_assert!(false, "ClippedRectangularStrip::from_lines insets/bay mismatch");
			insets.resize_with(bay_count, || None);
		}
		let mut strip = Self::new(style);
		strip.add_pair(rail_a[0], rail_b[0], None);
		for i in 0..bay_count {
			strip.add_pair(rail_a[i + 1], rail_b[i + 1], insets[i]);
		}
		strip
	}

	pub fn add_pair(
		&mut self,
		rail_a: impl Into<PanelPoint>,
		rail_b: impl Into<PanelPoint>,
		inset: Option<RectInset>,
	) -> &mut Self {
		let rail_a = rail_a.into();
		let rail_b = rail_b.into();

		if self.authored.is_empty() {
			self.authored.push((rail_a, rail_b));
			return self;
		}
		let (prev_a, prev_b) = *self.authored.last().unwrap();
		self.authored.push((rail_a, rail_b));

		match inset {
			None => {
				self.pieces.push(ClippedRectangularStripPiece::Solid(Rectangle::new(
					self.style, prev_a, rail_a, prev_b, rail_b,
				)));
			}
			Some(inset) => {
				self.pieces
					.push(ClippedRectangularStripPiece::Clipped(ClippedRectangle::new(
						self.style, prev_a, rail_a, prev_b, rail_b, inset,
					)));
			}
		}
		self
	}

	pub fn pieces(&self) -> &[ClippedRectangularStripPiece] {
		&self.pieces
	}

	pub fn authored_stations(&self) -> &[(PanelPoint, PanelPoint)] {
		&self.authored
	}
}

impl BuildingComponents for ClippedRectangularStrip {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			match piece {
				ClippedRectangularStripPiece::Solid(r) => {
					out.extend(r.panel_nodes_for_level(level));
				}
				ClippedRectangularStripPiece::Clipped(r) => {
					out.extend(r.panel_nodes_for_level(level));
				}
			}
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use richmond_building_components::panels::PanelGeometry;

	#[test]
	fn middle_inset_splits_pieces() {
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
		let s = ClippedRectangularStrip::from_lines(
			PanelStyle::RoughStonework,
			a,
			b,
			[Some(RectInset::uniform(0.35)), None],
		);
		assert_eq!(s.pieces().len(), 2);
		assert!(matches!(
			s.pieces()[0],
			ClippedRectangularStripPiece::Clipped(_)
		));
		assert!(matches!(s.pieces()[1], ClippedRectangularStripPiece::Solid(_)));
		let clipped = match &s.pieces()[0] {
			ClippedRectangularStripPiece::Clipped(c) => c,
			_ => unreachable!(),
		};
		assert_eq!(clipped.panels().len(), 4);
		assert!(clipped
			.panels()
			.iter()
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}
}
