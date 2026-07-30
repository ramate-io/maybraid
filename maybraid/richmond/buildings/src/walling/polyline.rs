//! Portal-sensitive polyline wall.
//!
//! \(t \in [0, 1]\) runs along cumulative **horizontal** path length through
//! [`PolylineWallParams::points`].

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::partitions::{
	wall_placement_from_centered, Partition, PartitionNode, PolylinePartition,
	DEFAULT_MIN_JOINT_ANGLE, DEFAULT_TILE_WIDTH, SLICE_KIT_HEIGHT,
};
use richmond_building_components::Placement;

use crate::walling::portal::{
	assign_portals, AssignedPortal, MustAssignPortal, Portal, PortalFootprint, WallRegion,
	SLICE_Y_FRAC,
};

/// Default portal opening width in world units.
pub const DEFAULT_PORTAL_WIDTH: f32 = 1.2;
const DEFAULT_THICK: f32 = 0.15 / 0.2;
const POLYLINE_SLOTS: u32 = 32;

/// Parameters for [`PolylineWall::new`].
#[derive(Debug, Clone)]
pub struct PolylineWallParams {
	pub points: Vec<Vec3>,
	pub height: f32,
	pub thickness: f32,
	pub portal_width: f32,
	/// Suggested tile width along each solid edge; fitted so \(n\) tiles span exactly.
	pub tile_width: f32,
	/// Omit joints when plan/slope kinks are below this (radians).
	pub min_joint_angle: f32,
	pub must_assign: Vec<MustAssignPortal>,
	pub must_not_assign: Vec<WallRegion>,
	pub portal_noise: NoiseParams,
	pub optional_portals: (u32, u32),
}

impl Default for PolylineWallParams {
	fn default() -> Self {
		Self {
			points: vec![
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 4.0),
			],
			height: 3.0,
			thickness: DEFAULT_THICK,
			portal_width: DEFAULT_PORTAL_WIDTH,
			tile_width: DEFAULT_TILE_WIDTH,
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
			must_assign: vec![],
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
		}
	}
}

/// Polyline wall with door/window openings.
#[derive(Debug, Clone, PartialEq)]
pub struct PolylineWall {
	pub points: Vec<Vec3>,
	pub height: f32,
	pub thickness: f32,
	pub portal_width: f32,
	pub tile_width: f32,
	pub min_joint_angle: f32,
	pub portals: Vec<AssignedPortal>,
	pub partitions: Vec<PartitionNode>,
}

impl PolylineWall {
	pub fn new(params: PolylineWallParams) -> Self {
		let height = params.height.max(1e-4);
		let thickness = params.thickness.max(1e-4);
		let portal_width = params.portal_width.max(1e-4);
		let tile_width = params.tile_width.max(1e-4);
		let min_joint_angle = params.min_joint_angle.max(0.0);
		let points = params.points;
		let total = path_length(&points).max(portal_width + 1e-3);
		let half_t = (portal_width * 0.5) / total;
		let noise = NoiseConfig::new(params.portal_noise);
		let foot = PortalFootprint { half_t, closed: false };

		let portals = assign_portals(
			&noise,
			&params.must_assign,
			&params.must_not_assign,
			params.optional_portals,
			foot,
			POLYLINE_SLOTS,
		);

		let partitions = tessellate_polyline(
			&points,
			height,
			thickness,
			portal_width,
			tile_width,
			min_joint_angle,
			&portals,
		);

		Self {
			points,
			height,
			thickness,
			portal_width,
			tile_width,
			min_joint_angle,
			portals,
			partitions,
		}
	}
}

fn path_length(points: &[Vec3]) -> f32 {
	points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn yaw_along(a: Vec3, b: Vec3) -> f32 {
	(-(b.z - a.z)).atan2(b.x - a.x)
}

fn sample_path(points: &[Vec3], t: f32) -> (Vec3, f32) {
	if points.len() < 2 {
		return (points.first().copied().unwrap_or(Vec3::ZERO), 0.0);
	}
	let total = path_length(points).max(1e-4);
	let mut target = t.clamp(0.0, 1.0) * total;
	for w in points.windows(2) {
		let len = w[0].distance(w[1]).max(1e-6);
		if target <= len + 1e-5 {
			let local = (target / len).clamp(0.0, 1.0);
			let p = w[0] + (w[1] - w[0]) * local;
			return (p, yaw_along(w[0], w[1]));
		}
		target -= len;
	}
	let last = points.len() - 1;
	(points[last], yaw_along(points[last - 1], points[last]))
}

fn subpath_points(points: &[Vec3], t0: f32, t1: f32) -> Vec<Vec3> {
	if points.len() < 2 || t1 <= t0 + 1e-6 {
		return vec![];
	}
	let total = path_length(points).max(1e-4);
	let s0 = t0.clamp(0.0, 1.0) * total;
	let s1 = t1.clamp(0.0, 1.0) * total;
	let mut out = Vec::new();
	let (p0, _) = sample_path(points, t0);
	out.push(p0);

	let mut acc = 0.0;
	for w in points.windows(2) {
		let len = w[0].distance(w[1]).max(1e-6);
		let seg_end = acc + len;
		if seg_end < s1 - 1e-4 && seg_end > s0 + 1e-4 {
			out.push(w[1]);
		}
		acc = seg_end;
	}

	let (p1, _) = sample_path(points, t1);
	if out.last().map(|p| p.distance(p1) > 1e-4).unwrap_or(true) {
		out.push(p1);
	}
	let _ = (s0, s1);
	out
}

fn tessellate_polyline(
	points: &[Vec3],
	height: f32,
	thickness: f32,
	portal_width: f32,
	tile_width: f32,
	min_joint_angle: f32,
	portals: &[AssignedPortal],
) -> Vec<PartitionNode> {
	if points.len() < 2 {
		return vec![];
	}

	let total = path_length(points).max(1e-4);
	let half_t = (portal_width * 0.5) / total;
	let mut partitions = Vec::new();

	for portal in portals {
		let (center, yaw) = sample_path(points, portal.t);
		let base = center;
		let lintel = base + Vec3::Y * (SLICE_Y_FRAC * height);
		let slice_half = portal_width * 0.5;
		let slice_height = SLICE_KIT_HEIGHT * height;
		match portal.portal {
			Portal::Door => {
				partitions.push(PartitionNode::rough_stone(
					Partition::linear(),
					wall_placement_from_centered(lintel, yaw, slice_half, slice_height, thickness),
				));
			}
			Portal::Window => {
				partitions.push(PartitionNode::rough_stone(
					Partition::linear(),
					wall_placement_from_centered(base, yaw, slice_half, slice_height, thickness),
				));
				partitions.push(PartitionNode::rough_stone(
					Partition::linear(),
					wall_placement_from_centered(lintel, yaw, slice_half, slice_height, thickness),
				));
			}
		}
	}

	let mut spans: Vec<(f32, f32)> = Vec::new();
	if portals.is_empty() {
		spans.push((0.0, 1.0));
	} else {
		spans.push((0.0, (portals[0].t - half_t).max(0.0)));
		for i in 0..portals.len().saturating_sub(1) {
			spans.push((portals[i].t + half_t, portals[i + 1].t - half_t));
		}
		spans.push(((portals[portals.len() - 1].t + half_t).min(1.0), 1.0));
	}

	for (t0, t1) in spans {
		if (t1 - t0) * total < 1e-2 {
			continue;
		}
		let sub = subpath_points(points, t0, t1);
		if sub.len() < 2 {
			continue;
		}
		let mut poly = PolylinePartition::new(sub)
			.with_tile_width(tile_width)
			.with_min_joint_angle(min_joint_angle)
			.with_wall_scale(height, thickness);
		if t0 > 1e-4 {
			let (p_prev, _) = sample_path(points, (t0 - 1e-3).max(0.0));
			let (p_at, _) = sample_path(points, t0);
			let d = p_at - p_prev;
			poly = poly.with_incoming_slope(
				richmond_building_components::partitions::roll_along_slope(d.x, d.y, d.z),
			);
		}
		// Identity parent: tiles carry world anchors, stand-up pitch, and wall scale.
		partitions.push(PartitionNode::rough_stone(Partition::Polyline(poly), Placement::IDENTITY));
	}

	partitions
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn l_path_with_door_emits_polyline_solids() -> anyhow::Result<()> {
		let wall = PolylineWall::new(PolylineWallParams {
			points: vec![
				Vec3::new(0.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 0.0),
				Vec3::new(4.0, 0.0, 4.0),
			],
			height: 3.0,
			must_assign: vec![MustAssignPortal::at(0.25, Portal::Door)],
			optional_portals: (0, 0),
			..PolylineWallParams::default()
		});
		assert_eq!(wall.portals.len(), 1);
		assert!(wall.partitions.iter().any(|p| matches!(p.geometry, Partition::Polyline(_))));
		assert!(wall.partitions.iter().any(|p| {
			matches!(p.geometry, Partition::Linear(_))
				&& (p.placement.scale.z - SLICE_KIT_HEIGHT * wall.height).abs() < 1e-3
		}));
		Ok(())
	}
}
