//! Private wall kit tessellation (not part of the public IR).

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::partitions::geometry::WallGeometry;
use crate::placed::{Placement, Placed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // kit pieces reserved for door frames / future tessellation
pub(crate) enum WallKit {
	Linear,
	LinearSubsegment,
	LinearHeaderSubsegment,
	Arc180,
	Arc90,
	Arc15,
	HeaderArc180,
	HeaderArc90,
	HeaderArc15,
}

impl From<ArcKit> for WallKit {
	fn from(kit: ArcKit) -> Self {
		match kit {
			ArcKit::D180 => Self::Arc180,
			ArcKit::D90 => Self::Arc90,
			ArcKit::D15 => Self::Arc15,
		}
	}
}

fn header_kit(kit: ArcKit) -> WallKit {
	match kit {
		ArcKit::D180 => WallKit::HeaderArc180,
		ArcKit::D90 => WallKit::HeaderArc90,
		ArcKit::D15 => WallKit::HeaderArc15,
	}
}

impl WallGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<WallKit>> {
		match self {
			Self::Linear(_) => vec![Placed::at_origin(WallKit::Linear)],
			Self::Polyline(g) => {
				let n = g.points.len().saturating_sub(1).max(1);
				(0..n).map(|_| Placed::at_origin(WallKit::Linear)).collect()
			}
			Self::Arc(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(WallKit::from(kit), bevy_math::Vec3::ZERO, yaw))
				.collect(),
			Self::HeaderArc(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(header_kit(kit), bevy_math::Vec3::ZERO, yaw))
				.collect(),
		}
	}

	pub(crate) fn placed_kits(&self, parent: Placement) -> Vec<Placed<WallKit>> {
		self.kit_pieces()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}
