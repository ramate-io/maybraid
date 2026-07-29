//! Private partition kit tessellation (not part of the public IR).

use bevy_math::Vec3;

use crate::arc_kit::{decompose_arc_sweep, ArcKit};
use crate::partitions::geometry::{PolylinePartition, PartitionGeometry};
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
	/// Circular joint at a plan / slope kink (high + mid LOD only).
	Joint,
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

/// Joint kit half-extent in \(X/Z\) (\([-0.5, 0.5]\)).
const JOINT_KIT_HALF: f32 = 0.5;
/// Base world radius when the kink is purely planar.
const JOINT_BASE_RADIUS: f32 = 0.15;
/// Extra world radius per radian of vertical (slope) kink.
const JOINT_RADIUS_PER_SLOPE_RAD: f32 = 0.55;
/// Default linear thickness scale (\(0.15\) world / \(0.2\) kit half-extent).
const DEFAULT_THICK: f32 = 0.15 / 0.2;

impl PartitionGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<PartitionKit>> {
		match self {
			Self::Linear(_) => vec![Placed::at_origin(PartitionKit::Linear)],
			Self::Polyline(g) => polyline_kit_pieces(g),
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

fn wrap_pi(mut a: f32) -> f32 {
	while a > std::f32::consts::PI {
		a -= std::f32::consts::TAU;
	}
	while a < -std::f32::consts::PI {
		a += std::f32::consts::TAU;
	}
	a
}

fn polyline_kit_pieces(poly: &PolylinePartition) -> Vec<Placed<PartitionKit>> {
	let points = &poly.points;
	if points.len() < 2 {
		return vec![Placed::at_origin(PartitionKit::Linear)];
	}

	let min_joint = poly.min_joint_angle.max(0.0);
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
		out.push(Placed {
			geom: PartitionKit::Linear,
			placement: Placement::new(mid, yaw)
				.with_roll(roll)
				.with_scale(Vec3::new(len * 0.5, 1.0, DEFAULT_THICK)),
		});
	}

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
		let dyaw = wrap_pi(yaw_out - yaw_in).abs();
		let droll = (roll_out - roll_in).abs();
		let kink = dyaw.max(droll);
		if kink < min_joint {
			continue;
		}

		// Kit \(X,Z \in [-0.5, 0.5]\) → world radius = scale * 0.5.
		// Grow horizontal size with vertical (slope) kink; plan-only turns use the base radius.
		let radius = JOINT_BASE_RADIUS + JOINT_RADIUS_PER_SLOPE_RAD * droll;
		let xz = (radius / JOINT_KIT_HALF).max(1e-4);
		// Tip along the average slope; yaw bisects the plan turn so roll stays in-plane.
		let yaw = yaw_in + 0.5 * wrap_pi(yaw_out - yaw_in);
		let roll = 0.5 * (roll_in + roll_out);
		out.push(Placed {
			geom: PartitionKit::Joint,
			placement: Placement::new(cur, yaw)
				.with_roll(roll)
				.with_scale(Vec3::new(xz, 1.0, xz)),
		});
	}

	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::partitions::geometry::DEFAULT_MIN_JOINT_ANGLE;

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
	fn small_plan_kink_omits_joint() -> anyhow::Result<()> {
		// ~0.05 rad turn — below default 0.1 threshold.
		let g = PartitionGeometry::Polyline(
			PolylinePartition::new([
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 0.1),
			])
			.with_min_joint_angle(DEFAULT_MIN_JOINT_ANGLE),
		);
		assert!(!g.kit_pieces().iter().any(|p| p.geom == PartitionKit::Joint));
		Ok(())
	}

	#[test]
	fn slope_kink_emits_joint_and_grows_radius() -> anyhow::Result<()> {
		let flat = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 2.0),
		]);
		let sloped = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(4.0, 2.0, 0.0),
		]);
		let flat_j = flat
			.kit_pieces()
			.into_iter()
			.find(|p| p.geom == PartitionKit::Joint)
			.ok_or_else(|| anyhow::anyhow!("flat joint"))?;
		let slope_j = sloped
			.kit_pieces()
			.into_iter()
			.find(|p| p.geom == PartitionKit::Joint)
			.ok_or_else(|| anyhow::anyhow!("slope joint"))?;
		assert!(
			slope_j.placement.scale.x > flat_j.placement.scale.x + 1e-3,
			"slope kink should grow joint radius"
		);
		// Flat in → slope out: roll ≈ 0.5 * atan2(2, 2) = π/8.
		let expected_roll = 0.5 * (2.0f32).atan2(2.0);
		assert!(
			(slope_j.placement.roll - expected_roll).abs() < 1e-3,
			"joint roll should average abutting slopes, got {}",
			slope_j.placement.roll
		);
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
