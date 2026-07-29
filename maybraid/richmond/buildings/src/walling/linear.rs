//! Portal-sensitive straight wall.
//!
//! \(t \in [0, 1]\) runs from [`LinearWallParams::start`] to [`LinearWallParams::end`].

use bevy_math::Vec3;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::partitions::{
	wall_placement_from_centered, LinearPartition, Partition, PartitionNode, DEFAULT_TILE_WIDTH,
	SLICE_KIT_HEIGHT,
};
use richmond_building_components::Placement;

use crate::walling::portal::{
	assign_portals, AssignedPortal, MustAssignPortal, Portal, PortalFootprint, WallRegion,
	SLICE_Y_FRAC,
};

/// Default portal opening width in world units.
pub const DEFAULT_PORTAL_WIDTH: f32 = 1.2;
/// Default kit thickness scale (\(0.15\) world / \(0.2\) kit half-extent).
const DEFAULT_THICK: f32 = 0.15 / 0.2;
/// Discrete candidate slots along an open linear wall.
const LINEAR_SLOTS: u32 = 24;

/// Parameters for [`LinearWall::new`].
#[derive(Debug, Clone)]
pub struct LinearWallParams {
	pub start: Vec3,
	pub end: Vec3,
	/// Full wall height (maps to panel kit \(Z\) after wall pitch).
	pub height: f32,
	/// Kit thickness scale along panel \(Y\) (default matches bedroom shells).
	pub thickness: f32,
	/// World-space portal opening width.
	pub portal_width: f32,
	/// Suggested solid-span tile width; fitted so \(n\) tiles span each cut exactly.
	pub tile_width: f32,
	pub must_assign: Vec<MustAssignPortal>,
	pub must_not_assign: Vec<WallRegion>,
	pub portal_noise: NoiseParams,
	pub optional_portals: (u32, u32),
}

impl Default for LinearWallParams {
	fn default() -> Self {
		Self {
			start: Vec3::new(-2.0, 0.0, 0.0),
			end: Vec3::new(2.0, 0.0, 0.0),
			height: 3.0,
			thickness: DEFAULT_THICK,
			portal_width: DEFAULT_PORTAL_WIDTH,
			tile_width: DEFAULT_TILE_WIDTH,
			must_assign: vec![],
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
		}
	}
}

/// Straight wall with door/window openings.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearWall {
	pub start: Vec3,
	pub end: Vec3,
	pub height: f32,
	pub thickness: f32,
	pub portal_width: f32,
	pub tile_width: f32,
	pub portals: Vec<AssignedPortal>,
	pub partitions: Vec<PartitionNode>,
}

impl LinearWall {
	pub fn new(params: LinearWallParams) -> Self {
		let height = params.height.max(1e-4);
		let thickness = params.thickness.max(1e-4);
		let portal_width = params.portal_width.max(1e-4);
		let tile_width = params.tile_width.max(1e-4);
		let length = horiz_len(params.start, params.end).max(portal_width + 1e-3);
		let half_t = (portal_width * 0.5) / length;
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
			LINEAR_SLOTS,
		);

		let partitions = tessellate_linear(
			params.start,
			params.end,
			height,
			thickness,
			portal_width,
			tile_width,
			&portals,
		);

		Self {
			start: params.start,
			end: params.end,
			height,
			thickness,
			portal_width,
			tile_width,
			portals,
			partitions,
		}
	}
}

fn horiz_len(a: Vec3, b: Vec3) -> f32 {
	let dx = b.x - a.x;
	let dz = b.z - a.z;
	(dx * dx + dz * dz).sqrt()
}

fn yaw_along(a: Vec3, b: Vec3) -> f32 {
	let dx = b.x - a.x;
	let dz = b.z - a.z;
	(-dz).atan2(dx)
}

fn point_at(start: Vec3, end: Vec3, t: f32) -> Vec3 {
	start + (end - start) * t
}

fn tessellate_linear(
	start: Vec3,
	end: Vec3,
	height: f32,
	thickness: f32,
	portal_width: f32,
	tile_width: f32,
	portals: &[AssignedPortal],
) -> Vec<PartitionNode> {
	let yaw = yaw_along(start, end);
	let length = horiz_len(start, end).max(1e-4);
	let half_t = (portal_width * 0.5) / length;
	let y0 = start.y.min(end.y);
	let mut partitions = Vec::new();

	for portal in portals {
		let center = point_at(start, end, portal.t);
		let base = Vec3::new(center.x, y0, center.z);
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

	let mut cuts: Vec<(f32, f32)> = Vec::new();
	if portals.is_empty() {
		cuts.push((0.0, 1.0));
	} else {
		cuts.push((0.0, (portals[0].t - half_t).max(0.0)));
		for i in 0..portals.len().saturating_sub(1) {
			let a = portals[i].t + half_t;
			let b = portals[i + 1].t - half_t;
			cuts.push((a, b));
		}
		cuts.push(((portals[portals.len() - 1].t + half_t).min(1.0), 1.0));
	}

	for (t0, t1) in cuts {
		let span = t1 - t0;
		let span_len = span * length;
		if span_len < 1e-2 {
			continue;
		}
		let a = point_at(start, end, t0);
		let b = point_at(start, end, t1);
		let mid = Vec3::new((a.x + b.x) * 0.5, y0, (a.z + b.z) * 0.5);
		partitions.push(PartitionNode::rough_stone(
			Partition::Linear(LinearPartition::spanning(span_len, tile_width)),
			// Child tiles own length; parent supplies thick (Y) and height (Z).
			Placement::new(mid, yaw).with_scale(Vec3::new(1.0, thickness, height)),
		));
	}

	partitions
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn door_splits_into_two_solids_and_slice() -> anyhow::Result<()> {
		let wall = LinearWall::new(LinearWallParams {
			start: Vec3::new(-4.0, 0.0, 0.0),
			end: Vec3::new(4.0, 0.0, 0.0),
			height: 3.0,
			must_assign: vec![MustAssignPortal::at(0.5, Portal::Door)],
			optional_portals: (0, 0),
			..LinearWallParams::default()
		});
		assert_eq!(wall.portals.len(), 1);
		let solids = wall
			.partitions
			.iter()
			.filter(|p| {
				matches!(p.geometry, Partition::Linear(_))
					&& (p.placement.scale.z - wall.height).abs() < 1e-3
			})
			.count();
		assert_eq!(solids, 2);
		let slices = wall
			.partitions
			.iter()
			.filter(|p| {
				matches!(p.geometry, Partition::Linear(_))
					&& (p.placement.scale.z - SLICE_KIT_HEIGHT * wall.height).abs() < 1e-3
			})
			.count();
		assert_eq!(slices, 1);
		Ok(())
	}
}
