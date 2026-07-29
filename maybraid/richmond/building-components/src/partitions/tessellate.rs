//! Private partition kit tessellation (not part of the public IR).

use bevy_math::Vec3;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::partitions::geometry::PartitionGeometry;
use crate::placed::{Placement, Placed};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)] // kit pieces reserved for door frames / future tessellation
pub(crate) enum PartitionKit {
	Linear,
	LinearSubsegment,
	LinearHeaderSubsegment,
	Arc180,
	Arc90,
	Arc15,
	HeaderArc180,
	HeaderArc90,
	HeaderArc15,
	/// Horizontal joint at a plan-angle vertex (empty scene placeholder).
	Joint,
	/// Vertical wedge at an elevation kink (empty scene placeholder).
	Wedge,
}

impl From<ArcKit> for PartitionKit {
	fn from(kit: ArcKit) -> Self {
		match kit {
			ArcKit::D180 => Self::Arc180,
			ArcKit::D90 => Self::Arc90,
			ArcKit::D15 => Self::Arc15,
		}
	}
}

fn header_kit(kit: ArcKit) -> PartitionKit {
	match kit {
		ArcKit::D180 => PartitionKit::HeaderArc180,
		ArcKit::D90 => PartitionKit::HeaderArc90,
		ArcKit::D15 => PartitionKit::HeaderArc15,
	}
}

/// Minimum horizontal turn (radians) to emit a joint kit.
const JOINT_YAW_EPS: f32 = 1e-3;
/// Minimum elevation difference / slope roll (radians) to emit a wedge kit.
const WEDGE_Y_EPS: f32 = 1e-3;
const WEDGE_ROLL_EPS: f32 = 1e-3;
/// Default kit thickness scale (\(0.15\) world / \(0.2\) kit half-extent).
const DEFAULT_THICK: f32 = 0.15 / 0.2;

impl PartitionGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<PartitionKit>> {
		match self {
			Self::Linear(_) => vec![Placed::at_origin(PartitionKit::Linear)],
			Self::Polyline(g) => polyline_kit_pieces(&g.points),
			Self::Arc(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(PartitionKit::from(kit), Vec3::ZERO, yaw))
				.collect(),
			Self::HeaderArc(g) => decompose_arc_sweep(g.sweep_degrees)
				.into_iter()
				.map(|(kit, yaw)| Placed::new(header_kit(kit), Vec3::ZERO, yaw))
				.collect(),
		}
	}

	pub(crate) fn placed_kits(&self, parent: Placement) -> Vec<Placed<PartitionKit>> {
		self.kit_pieces()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}

/// Yaw that aligns kit local \(+X\) with horizontal direction \((\mathrm{d}x, \mathrm{d}z)\).
fn yaw_along_xz(dx: f32, dz: f32) -> f32 {
	(-dz).atan2(dx)
}

/// Roll about local \(+Z\) so kit \(+X\) follows the path slope (in the wall plane).
fn roll_along_slope(dx: f32, dy: f32, dz: f32) -> f32 {
	let horiz = (dx * dx + dz * dz).sqrt();
	dy.atan2(horiz.max(1e-8))
}

fn polyline_kit_pieces(points: &[Vec3]) -> Vec<Placed<PartitionKit>> {
	if points.len() < 2 {
		return vec![Placed::at_origin(PartitionKit::Linear)];
	}

	let mut out = Vec::new();
	let n_edges = points.len() - 1;

	for i in 0..n_edges {
		let a = points[i];
		let b = points[i + 1];
		let delta = b - a;
		let len = delta.length().max(1e-4);
		let yaw = yaw_along_xz(delta.x, delta.z);
		let roll = roll_along_slope(delta.x, delta.y, delta.z);
		let mid = (a + b) * 0.5;
		// Unit kit height: parent `Placement.scale.y` supplies storey height.
		// Slope uses roll about local Z (not pitch — pitch would lean the face out).
		out.push(Placed {
			geom: PartitionKit::Linear,
			placement: Placement::new(mid, yaw)
				.with_roll(roll)
				.with_scale(Vec3::new(len * 0.5, 1.0, DEFAULT_THICK)),
		});
	}

	// Interior vertices: joints (plan turn) and wedges (elevation kink).
	for i in 1..points.len() - 1 {
		let prev = points[i - 1];
		let cur = points[i];
		let next = points[i + 1];
		let din = cur - prev;
		let dout = next - cur;
		let yaw_in = yaw_along_xz(din.x, din.z);
		let yaw_out = yaw_along_xz(dout.x, dout.z);
		let roll_in = roll_along_slope(din.x, din.y, din.z);
		let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
		let mut dyaw = yaw_out - yaw_in;
		while dyaw > std::f32::consts::PI {
			dyaw -= std::f32::consts::TAU;
		}
		while dyaw < -std::f32::consts::PI {
			dyaw += std::f32::consts::TAU;
		}

		if dyaw.abs() > JOINT_YAW_EPS {
			out.push(Placed {
				geom: PartitionKit::Joint,
				placement: Placement::new(cur, yaw_out)
					.with_roll(roll_out)
					.with_scale(Vec3::new(DEFAULT_THICK, 1.0, DEFAULT_THICK)),
			});
		}

		let elev_kink = (cur.y - prev.y).abs() > WEDGE_Y_EPS
			|| (next.y - cur.y).abs() > WEDGE_Y_EPS
			|| (roll_out - roll_in).abs() > WEDGE_ROLL_EPS;
		if elev_kink {
			out.push(Placed {
				geom: PartitionKit::Wedge,
				placement: Placement::new(cur, yaw_out)
					.with_roll(0.5 * (roll_in + roll_out))
					.with_scale(Vec3::new(DEFAULT_THICK, 1.0, DEFAULT_THICK)),
			});
		}
	}

	out
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn straight_polyline_has_no_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(4.0, 0.0, 0.0),
		]);
		let pieces = g.kit_pieces();
		assert_eq!(
			pieces
				.iter()
				.filter(|p| p.geom == PartitionKit::Linear)
				.count(),
			2
		);
		assert!(!pieces.iter().any(|p| p.geom == PartitionKit::Joint));
		assert!(!pieces.iter().any(|p| p.geom == PartitionKit::Wedge));
		Ok(())
	}

	#[test]
	fn l_shape_emits_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 2.0),
		]);
		let pieces = g.kit_pieces();
		assert_eq!(
			pieces
				.iter()
				.filter(|p| p.geom == PartitionKit::Joint)
				.count(),
			1
		);
		Ok(())
	}

	#[test]
	fn stepped_height_emits_wedge() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(4.0, 1.0, 0.0),
		]);
		let pieces = g.kit_pieces();
		assert!(pieces.iter().any(|p| p.geom == PartitionKit::Wedge));
		Ok(())
	}

	#[test]
	fn sloping_edge_sets_roll() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 1.0, 0.0),
		]);
		let pieces = g.kit_pieces();
		let linear = pieces
			.iter()
			.find(|p| p.geom == PartitionKit::Linear)
			.ok_or_else(|| anyhow::anyhow!("missing linear"))?;
		assert!(
			linear.placement.roll.abs() > 0.2,
			"expected rolled segment along slope, roll={}",
			linear.placement.roll
		);
		assert!(
			linear.placement.pitch.abs() < 1e-5,
			"polyline slope must not lean the face (pitch)"
		);
		assert!((linear.placement.translation.y - 0.5).abs() < 1e-3);
		Ok(())
	}
}
