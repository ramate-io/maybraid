//! Private floor kit tessellation (not part of the public IR).

use scene_ref::MirrorAxis;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::floors::geometry::FloorGeometry;
use crate::floors::style::FloorStyle;
use crate::panels::{
	PanelGeom, QuadPolyline, Rectangle, RightTriangle, TessellatePolicy,
};
use crate::placed::{Placement, Placed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FloorKit {
	Rectangle,
	RightTriangle {
		mirror: Option<MirrorAxis>,
	},
	ArcFill(ArcKit),
	StructFill,
	CircleInscribedSquare,
	/// Partition-style joint (omitted for styles without a joint leaf).
	Joint,
}

impl FloorGeometry {
	pub(crate) fn tessellate_policy(&self, style: FloorStyle) -> TessellatePolicy {
		match style {
			FloorStyle::RoughStonework | FloorStyle::Wood => TessellatePolicy::RECTANGLE,
		}
	}

	pub(crate) fn kit_pieces_with_policy(&self, policy: TessellatePolicy) -> Vec<Placed<FloorKit>> {
		match self {
			Self::Rectangle(_) => vec![Placed::at_origin(FloorKit::Rectangle)],
			Self::StructFill(_) => vec![Placed::at_origin(FloorKit::StructFill)],
			Self::CircleInscribedSquare(_) => {
				vec![Placed::at_origin(FloorKit::CircleInscribedSquare)]
			}
			Self::ArcFill(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(FloorKit::ArcFill(kit), bevy_math::Vec3::ZERO, yaw))
				.collect(),
			Self::Quad(q) => map_panel_atoms(q.decompose(policy)),
			Self::QuadPolyline(pl) => expand_quad_polyline(pl, policy),
		}
	}

	pub(crate) fn placed_kits_for_style(
		&self,
		style: FloorStyle,
		parent: Placement,
	) -> Vec<Placed<FloorKit>> {
		self.kit_pieces_with_policy(self.tessellate_policy(style))
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}

fn expand_quad_polyline(pl: &QuadPolyline, policy: TessellatePolicy) -> Vec<Placed<FloorKit>> {
	let mut out = Vec::new();
	for piece in pl.decompose() {
		match piece.geom {
			PanelGeom::Quad(q) => {
				for child in q.decompose(policy) {
					if let Some(kit) = panel_to_floor(child.geom) {
						out.push(Placed {
							geom: kit,
							placement: piece.placement.compose_child(child.placement),
						});
					}
				}
			}
			PanelGeom::Joint(_) => {
				out.push(Placed {
					geom: FloorKit::Joint,
					placement: piece.placement,
				});
			}
			other => {
				if let Some(kit) = panel_to_floor(other) {
					out.push(Placed {
						geom: kit,
						placement: piece.placement,
					});
				}
			}
		}
	}
	out
}

fn map_panel_atoms(pieces: Vec<Placed<PanelGeom>>) -> Vec<Placed<FloorKit>> {
	pieces
		.into_iter()
		.filter_map(|p| {
			panel_to_floor(p.geom).map(|kit| Placed {
				geom: kit,
				placement: p.placement,
			})
		})
		.collect()
}

fn panel_to_floor(geom: PanelGeom) -> Option<FloorKit> {
	match geom {
		PanelGeom::Rectangle(Rectangle) => Some(FloorKit::Rectangle),
		PanelGeom::RightTriangle(RightTriangle { mirror }) => {
			Some(FloorKit::RightTriangle { mirror })
		}
		_ => None,
	}
}
