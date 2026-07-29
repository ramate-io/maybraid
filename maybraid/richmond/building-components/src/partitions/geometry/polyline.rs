//! Short-run polyline partition (single LOD parent) and tessellation into tiles.

use bevy_math::{Vec2, Vec3};

use crate::partitions::geometry::joint::JointPartition;
use crate::partitions::geometry::linear::DEFAULT_THICK;
use crate::partitions::geometry::PartitionTile;
use crate::placed::{Placed, Placement};

/// Default polyline joint omission threshold (radians).
pub const DEFAULT_MIN_JOINT_ANGLE: f32 = 0.1;

/// Short-run polyline. Prefer splitting long paths in higher-order walling/buildings.
///
/// One [`crate::partitions::PartitionNode`] is a single LOD parent for all kits.
#[derive(Debug, Clone, PartialEq)]
pub struct PolylinePartition {
	pub points: Vec<Vec3>,
	/// Omit joint kits when both plan and slope kink angles are below this (radians).
	pub min_joint_angle: f32,
	/// Roll of the segment that ends at `points[0]` but is not part of this polyline.
	/// When set, evaluate a start joint at `points[0]` against the first edge.
	pub incoming_slope: Option<f32>,
}

impl Default for PolylinePartition {
	fn default() -> Self {
		Self {
			points: Vec::new(),
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
			incoming_slope: None,
		}
	}
}

impl PolylinePartition {
	pub fn new(points: impl Into<Vec<Vec3>>) -> Self {
		Self {
			points: points.into(),
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
			incoming_slope: None,
		}
	}

	pub fn with_min_joint_angle(mut self, min_joint_angle: f32) -> Self {
		self.min_joint_angle = min_joint_angle.max(0.0);
		self
	}

	pub fn with_incoming_slope(mut self, incoming_slope: f32) -> Self {
		self.incoming_slope = Some(incoming_slope);
		self
	}

	/// Expand into posed linear + joint tiles (identity parent).
	pub fn tiles(&self) -> Vec<Placed<PartitionTile>> {
		let points = &self.points;
		if points.len() < 2 {
			return vec![Placed::at_origin(PartitionTile::Linear)];
		}

		let min_joint = self.min_joint_angle.max(0.0);
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
				geom: PartitionTile::Linear,
				placement: Placement::new(mid, yaw)
					.with_roll(roll)
					.with_scale(Vec3::new(len * 0.5, 1.0, DEFAULT_THICK)),
			});
		}

		if let Some(roll_in) = self.incoming_slope {
			let a = points[0];
			let b = points[1];
			let dout = b - a;
			let yaw_out = yaw_along_xz(dout.x, dout.z);
			let roll_out = roll_along_slope(dout.x, dout.y, dout.z);
			let droll = (roll_out - roll_in).abs();
			if droll >= min_joint {
				out.push(JointPartition::placed_at(
					a, yaw_out, yaw_out, roll_in, roll_out,
				));
			}
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
			out.push(JointPartition::placed_at(
				cur, yaw_in, yaw_out, roll_in, roll_out,
			));
		}

		out
	}
}

/// Convert legacy 2D polyline points (XZ) into 3D with \(Y = 0\).
pub fn polyline_from_xz(points: impl IntoIterator<Item = Vec2>) -> PolylinePartition {
	PolylinePartition::new(
		points
			.into_iter()
			.map(|p| Vec3::new(p.x, 0.0, p.y))
			.collect::<Vec<_>>(),
	)
}

pub(crate) fn yaw_along_xz(dx: f32, dz: f32) -> f32 {
	(-dz).atan2(dx)
}

/// Slope roll about local \(+Z\) for an edge \(\Delta = (\mathrm{d}x,\mathrm{d}y,\mathrm{d}z)\).
pub fn roll_along_slope(dx: f32, dy: f32, dz: f32) -> f32 {
	let horiz = (dx * dx + dz * dz).sqrt();
	dy.atan2(horiz.max(1e-8))
}

pub(crate) fn wrap_pi(mut a: f32) -> f32 {
	while a > std::f32::consts::PI {
		a -= std::f32::consts::TAU;
	}
	while a < -std::f32::consts::PI {
		a += std::f32::consts::TAU;
	}
	a
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::partitions::geometry::PartitionGeometry;

	#[test]
	fn straight_polyline_has_no_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(4.0, 0.0, 0.0),
		]);
		let pieces = g.tiles();
		assert_eq!(
			pieces
				.iter()
				.filter(|p| p.geom == PartitionTile::Linear)
				.count(),
			2
		);
		assert!(!pieces.iter().any(|p| p.geom == PartitionTile::Joint));
		Ok(())
	}

	#[test]
	fn l_shape_emits_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::polyline([
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 2.0),
		]);
		assert_eq!(
			g.tiles()
				.iter()
				.filter(|p| p.geom == PartitionTile::Joint)
				.count(),
			1
		);
		Ok(())
	}

	#[test]
	fn small_plan_kink_omits_joint() -> anyhow::Result<()> {
		let g = PartitionGeometry::Polyline(
			PolylinePartition::new([
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(2.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 0.1),
			])
			.with_min_joint_angle(DEFAULT_MIN_JOINT_ANGLE),
		);
		assert!(!g.tiles().iter().any(|p| p.geom == PartitionTile::Joint));
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
			.tiles()
			.into_iter()
			.find(|p| p.geom == PartitionTile::Joint)
			.ok_or_else(|| anyhow::anyhow!("flat joint"))?;
		let slope_j = sloped
			.tiles()
			.into_iter()
			.find(|p| p.geom == PartitionTile::Joint)
			.ok_or_else(|| anyhow::anyhow!("slope joint"))?;
		assert!(
			slope_j.placement.scale.x > flat_j.placement.scale.x + 1e-3,
			"slope kink should grow joint radius"
		);
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
		let linear = g
			.tiles()
			.into_iter()
			.find(|p| p.geom == PartitionTile::Linear)
			.ok_or_else(|| anyhow::anyhow!("missing linear"))?;
		assert!(linear.placement.roll.abs() > 0.2);
		assert!(linear.placement.pitch.abs() < 1e-5);
		assert!((linear.placement.translation.y - 0.5).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn incoming_slope_emits_start_joint() -> anyhow::Result<()> {
		let flat_in = PartitionGeometry::Polyline(
			PolylinePartition::new([Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0)])
				.with_incoming_slope(0.0),
		);
		assert_eq!(
			flat_in
				.tiles()
				.iter()
				.filter(|p| p.geom == PartitionTile::Joint)
				.count(),
			1
		);
		let matched = PartitionGeometry::Polyline(
			PolylinePartition::new([Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 1.0, 0.0)])
				.with_incoming_slope(roll_along_slope(2.0, 1.0, 0.0)),
		);
		assert!(!matched
			.tiles()
			.iter()
			.any(|p| p.geom == PartitionTile::Joint));
		Ok(())
	}
}
