//! Private floor kit tessellation (not part of the public IR).

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::floors::geometry::FloorGeometry;
use crate::placed::{Placement, Placed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FloorKit {
	Rectangle,
	ArcFill(ArcKit),
	StructFill,
	CircleInscribedSquare,
}

impl FloorGeometry {
	/// Expand continuous geometry into placed kit pieces (local to the form).
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<FloorKit>> {
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
		}
	}

	pub(crate) fn placed_kits(&self, parent: Placement) -> Vec<Placed<FloorKit>> {
		self.kit_pieces()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}
