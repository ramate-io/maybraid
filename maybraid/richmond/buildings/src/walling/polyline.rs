//! Portal-sensitive polyline wall.
//!
//! \(t \in [0, 1]\) runs along cumulative **horizontal** path length through
//! [`PolylineWallParams::points`].

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::partitions::{Partition, PartitionNode, HEADER_KIT_HEIGHT};
use richmond_building_components::Placement;

use crate::walling::portal::{
	assign_portals, AssignedPortal, MustAssignPortal, Portal, PortalFootprint, WallRegion,
	HEADER_Y_FRAC,
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
	pub portals: Vec<AssignedPortal>,
	pub partitions: Vec<PartitionNode>,
}

impl PolylineWall {
	pub fn new(params: PolylineWallParams) -> Self {
		let height = params.height.max(1e-4);
		let thickness = params.thickness.max(1e-4);
		let portal_width = params.portal_width.max(1e-4);
		let points = params.points;
		let total = path_length(&points).max(portal_width + 1e-3);
		let half_t = (portal_width * 0.5) / total;
		let noise = NoiseConfig::new(params.portal_noise);
		let foot = PortalFootprint {
			half_t,
			closed: false,
		};

		let portals = assign_portals(
			&noise,
			&params.must_assign,
			&params.must_not_assign,
			params.optional_portals,
			foot,
			POLYLINE_SLOTS,
		);

		let partitions = tessellate_polyline(&points, height, thickness, portal_width, &portals);

		Self {
			points,
			height,
			thickness,
			portal_width,
			portals,
			partitions,
		}
	}
}

fn horiz_dist(a: Vec3, b: Vec3) -> f32 {
	let dx = b.x - a.x;
	let dz = b.z - a.z;
	(dx * dx + dz * dz).sqrt()
}

fn path_length(points: &[Vec3]) -> f32 {
	points.windows(2).map(|w| horiz_dist(w[0], w[1])).sum()
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
		let len = horiz_dist(w[0], w[1]).max(1e-6);
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
		let len = horiz_dist(w[0], w[1]).max(1e-6);
		let seg_end = acc + len;
		if seg_end < s1 - 1e-4 && seg_end > s0 + 1e-4 {
			out.push(w[1]);
		}
		acc = seg_end;
	}

	let (p1, _) = sample_path(points, t1);
	if out
		.last()
		.map(|p| horiz_dist(*p, p1) > 1e-4)
		.unwrap_or(true)
	{
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
	portals: &[AssignedPortal],
) -> Vec<PartitionNode> {
	if points.len() < 2 {
		return vec![];
	}

	let total = path_length(points).max(1e-4);
	let half_t = (portal_width * 0.5) / total;
	let thick_scale = thickness / DEFAULT_THICK;
	let mut partitions = Vec::new();

	for portal in portals {
		let (center, yaw) = sample_path(points, portal.t);
		let y0 = center.y;
		let base = Vec3::new(center.x, y0, center.z);
		let lintel = base + Vec3::Y * (HEADER_Y_FRAC * height);
		let header_scale = Vec3::new(portal_width * 0.5, HEADER_KIT_HEIGHT * height, thickness);
		match portal.portal {
			Portal::Door => {
				partitions.push(PartitionNode::rough_stone(
					Partition::linear(),
					Placement::new(lintel, yaw).with_scale(header_scale),
				));
			}
			Portal::Window => {
				partitions.push(PartitionNode::rough_stone(
					Partition::linear(),
					Placement::new(base, yaw).with_scale(header_scale),
				));
				partitions.push(PartitionNode::rough_stone(
					Partition::linear(),
					Placement::new(lintel, yaw).with_scale(header_scale),
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
		let mut sub = subpath_points(points, t0, t1);
		if sub.len() < 2 {
			continue;
		}
		let y0 = sub.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
		for p in &mut sub {
			p.y = y0;
		}
		// Unit kit height × parent Y scale; thickness via parent Z scale.
		partitions.push(PartitionNode::rough_stone(
			Partition::polyline(sub),
			Placement::at_origin().with_scale(Vec3::new(1.0, height, thick_scale)),
		));
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
		assert!(wall
			.partitions
			.iter()
			.any(|p| matches!(p.geometry, Partition::Polyline(_))));
		assert!(wall.partitions.iter().any(|p| {
			matches!(p.geometry, Partition::Linear(_))
				&& (p.placement.scale.y - HEADER_KIT_HEIGHT * wall.height).abs() < 1e-3
		}));
		Ok(())
	}
}
