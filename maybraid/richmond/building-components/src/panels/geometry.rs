//! Shared panel geometry IR and kit capabilities.

use scene_ref::MirrorAxis;

use crate::floors::style::FloorStyle;
use crate::partitions::style::PartitionStyle;
use crate::placed::{Placed, Placement};
use crate::roofs::style::RoofStyle;

/// Suggested full tile width along local \(X\) (matches unscaled ground kit \(X \in [0, 1]\)).
pub const DEFAULT_TILE_WIDTH: f32 = 1.0;

/// How many tiles fit a length given a suggested width.
///
/// \(n = \mathrm{round}(\texttt{length}/\texttt{tile\_width})\), at least 1. Callers use
/// \(\texttt{length}/n\) as the actual tile size so tiles span the length exactly.
pub fn fitted_tile_count(length: f32, tile_width: f32) -> u32 {
	let tw = tile_width.max(1e-4);
	((length / tw).round() as i32).max(1) as u32
}

/// Kit capabilities for a panel look (not a user tessellation preference).
///
/// When [`Self::has_rectangle`] is false, rectangular body regions are filled with
/// complementary right-triangle pairs. Domain / panel material styles map into this
/// via [`From`]; see [`crate::panels::PanelStyle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelKitCaps {
	pub has_rectangle: bool,
}

impl PanelKitCaps {
	pub const WITH_RECTANGLE: Self = Self { has_rectangle: true };
	pub const TRIANGLES_ONLY: Self = Self { has_rectangle: false };
}

impl From<RoofStyle> for PanelKitCaps {
	fn from(style: RoofStyle) -> Self {
		match style {
			RoofStyle::ShepherdsThatch => Self::TRIANGLES_ONLY,
		}
	}
}

impl From<PartitionStyle> for PanelKitCaps {
	fn from(style: PartitionStyle) -> Self {
		match style {
			PartitionStyle::RoughStonework => Self::WITH_RECTANGLE,
		}
	}
}

impl From<FloorStyle> for PanelKitCaps {
	fn from(style: FloorStyle) -> Self {
		match style {
			FloorStyle::RoughStonework | FloorStyle::Wood => Self::WITH_RECTANGLE,
		}
	}
}

/// Axis-aligned rectangular panel tile in panel space.
///
/// Kit footprint: \(X \in [0, 1]\), \(Z \in [-1, 0]\) (lower-left at origin, depth toward \(-Z\)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rectangle;

/// Unit right-triangle panel atom.
///
/// Kit footprint: \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\)
/// (right angle at the origin; third corner at local \((0, 0, -1)\)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RightTriangle {
	pub mirror: Option<MirrorAxis>,
}

impl Default for RightTriangle {
	fn default() -> Self {
		Self { mirror: None }
	}
}

impl RightTriangle {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn mirrored(mirror: MirrorAxis) -> Self {
		Self { mirror: Some(mirror) }
	}
}

/// Shared panel geometry IR. All variants can [`PanelGeometry::decompose`].
///
/// Composition:
/// - tessellated triangles ← [`RightTriangle`] / [`crate::panels::TessellatedTriangle`]
/// - tessellated rectangles ← [`Rectangle`] (+ triangles when `!has_rectangle`)
#[derive(Debug, Clone, PartialEq)]
pub enum PanelGeometry {
	RightTriangle(RightTriangle),
	Rectangle(Rectangle),
	TessellatedTriangle(crate::panels::TessellatedTriangle),
}

impl PanelGeometry {
	pub fn rectangle() -> Self {
		Self::Rectangle(Rectangle)
	}

	pub fn right_triangle(mirror: Option<MirrorAxis>) -> Self {
		Self::RightTriangle(RightTriangle { mirror })
	}

	pub fn tessellated_triangle(t: crate::panels::TessellatedTriangle) -> Self {
		Self::TessellatedTriangle(t)
	}

	/// One-level decompose toward simpler variants.
	pub fn decompose(&self, caps: PanelKitCaps) -> Vec<Placed<PanelGeometry>> {
		match self {
			Self::RightTriangle(t) => {
				vec![Placed::at_origin(Self::RightTriangle(*t))]
			}
			Self::Rectangle(r) => {
				if caps.has_rectangle {
					vec![Placed::at_origin(Self::Rectangle(*r))]
				} else {
					// Unit square \(X \in [0,1]\), \(Z \in [-1,0]\) as dual kits
					// (identity + complement at lower-right with yaw π).
					use bevy_math::Vec3;
					use std::f32::consts::PI;
					vec![
						Placed::with_placement(
							Self::RightTriangle(RightTriangle { mirror: None }),
							Placement::new(Vec3::ZERO, 0.0),
						),
						Placed::with_placement(
							Self::RightTriangle(RightTriangle { mirror: None }),
							Placement::new(Vec3::new(1.0, 0.0, -1.0), PI),
						),
					]
				}
			}
			Self::TessellatedTriangle(t) => t.decompose(),
		}
	}

	/// Flatten composites to leaf atoms ([`Rectangle`], [`RightTriangle`]).
	pub fn flatten(&self, caps: PanelKitCaps) -> Vec<Placed<PanelGeometry>> {
		flatten_placed(Placed::at_origin(self.clone()), caps)
	}

	pub fn is_leaf_atom(&self) -> bool {
		matches!(self, Self::Rectangle(_) | Self::RightTriangle(_))
	}
}

fn flatten_placed(placed: Placed<PanelGeometry>, caps: PanelKitCaps) -> Vec<Placed<PanelGeometry>> {
	if placed.geom.is_leaf_atom() {
		// Rectangle may still expand to dual triangles under kit caps.
		if matches!(placed.geom, PanelGeometry::Rectangle(_)) && !caps.has_rectangle {
			return placed
				.geom
				.decompose(caps)
				.into_iter()
				.map(|child| Placed {
					geom: child.geom,
					placement: placed.placement.compose_child(child.placement),
				})
				.collect();
		}
		return vec![placed];
	}
	placed
		.geom
		.decompose(caps)
		.into_iter()
		.flat_map(|child| {
			flatten_placed(
				Placed {
					geom: child.geom,
					placement: placed.placement.compose_child(child.placement),
				},
				caps,
			)
		})
		.collect()
}
