//! Parameterized arc wall with portal (door/window) openings.
//!
//! \(t \in [0, 1]\) runs along the wall’s covered sweep ([`ArcWallParams::arc_degrees`]),
//! not necessarily a full circle.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use richmond_building_components::partitions::{Partition, PartitionNode};
use richmond_building_components::{BuildingComponents, Placement};

use crate::walling::portal::{
	assign_portals, AssignedPortal, MustAssignPortal, Portal, PortalFootprint, WallRegion,
	SLICE_Y_FRAC,
};

/// Kit segment size (degrees) and portal width (two segments → 30°).
const SEG_DEG: f32 = 15.0;
const PORTAL_SEGS: u32 = 2;
/// Half-width of each portal in degrees (two 15° segments → 30° total).
const OPEN_HALF_DEG: f32 = SEG_DEG * (PORTAL_SEGS as f32) * 0.5;

/// Parameters for [`ArcWall::new`].
#[derive(Debug, Clone)]
pub struct ArcWallParams {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	/// Degrees of arc this wall covers (\((0, 360]\); \(360\) is a closed ring).
	pub arc_degrees: f32,
	/// Regions that **must** receive a portal (best-fit). \(t\) is along this arc.
	pub must_assign: Vec<MustAssignPortal>,
	/// Regions that **must not** receive a portal.
	pub must_not_assign: Vec<WallRegion>,
	/// Noise used for optional portal count and placement.
	pub portal_noise: NoiseParams,
	/// Inclusive \((min, max)\) optional portals to attempt in can-assign space.
	pub optional_portals: (u32, u32),
}

/// Arc-shaped wall with door/window openings.
#[derive(Debug, Clone, PartialEq)]
pub struct ArcWall {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	pub arc_degrees: f32,
	pub portals: Vec<AssignedPortal>,
	pub partitions: Vec<PartitionNode>,
}

impl ArcWall {
	/// Assign must portals, then noise-sample optional portals in can-assign regions.
	pub fn new(params: ArcWallParams) -> Self {
		let radius = params.radius.max(1e-4);
		let storey_height = params.storey_height.max(1e-4);
		let arc_degrees = params.arc_degrees.clamp(SEG_DEG, 360.0);
		let noise = NoiseConfig::new(params.portal_noise);
		let closed = is_closed(arc_degrees);
		let slots = seg_count(arc_degrees);
		let half_t = OPEN_HALF_DEG / arc_degrees.max(SEG_DEG);
		let foot = PortalFootprint { half_t, closed };

		let portals = assign_portals(
			&noise,
			&params.must_assign,
			&params.must_not_assign,
			params.optional_portals,
			foot,
			slots,
		);

		let partitions =
			tessellate_arc(params.center_xz, radius, storey_height, arc_degrees, closed, &portals);
		Self {
			center_xz: params.center_xz,
			radius,
			storey_height,
			arc_degrees,
			portals,
			partitions,
		}
	}
}

impl BuildingComponents for ArcWall {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Vec<PartitionNode> {
		self.partitions.clone()
	}
}


fn is_closed(arc_degrees: f32) -> bool {
	(arc_degrees - 360.0).abs() < 0.5
}

fn seg_count(arc_degrees: f32) -> u32 {
	((arc_degrees / SEG_DEG).round() as u32).max(1)
}

fn tessellate_arc(
	center_xz: Vec3,
	radius: f32,
	storey_height: f32,
	arc_degrees: f32,
	closed: bool,
	portals: &[AssignedPortal],
) -> Vec<PartitionNode> {
	let ring_scale = Vec3::new(radius, storey_height, radius);
	let lintel = center_xz + Vec3::Y * (SLICE_Y_FRAC * storey_height);
	let mut partitions = Vec::new();

	for portal in portals {
		let center_deg = portal.t * arc_degrees;
		let open_start = center_deg - OPEN_HALF_DEG;
		for i in 0..PORTAL_SEGS {
			let seg_start = if closed {
				norm_deg(open_start + i as f32 * SEG_DEG)
			} else {
				(open_start + i as f32 * SEG_DEG).clamp(0.0, arc_degrees - SEG_DEG)
			};
			let yaw = seg_start.to_radians();
			match portal.portal {
				Portal::Door => {
					partitions.push(PartitionNode::rough_stone(
						Partition::slice_arc(SEG_DEG),
						Placement::new(lintel, yaw).with_scale(ring_scale),
					));
				}
				Portal::Window => {
					partitions.push(PartitionNode::rough_stone(
						Partition::slice_arc(SEG_DEG),
						Placement::new(center_xz, yaw).with_scale(ring_scale),
					));
					partitions.push(PartitionNode::rough_stone(
						Partition::slice_arc(SEG_DEG),
						Placement::new(lintel, yaw).with_scale(ring_scale),
					));
				}
			}
		}
	}

	if portals.is_empty() {
		push_solid_sweep(&mut partitions, center_xz, ring_scale, 0.0, arc_degrees);
		return partitions;
	}

	if closed {
		for i in 0..portals.len() {
			let c0 = portals[i].t * arc_degrees;
			let c1 = portals[(i + 1) % portals.len()].t * arc_degrees;
			let solid_start = norm_deg(c0 + OPEN_HALF_DEG);
			let solid_end = norm_deg(c1 - OPEN_HALF_DEG);
			let sweep = if solid_end >= solid_start - 1e-3 {
				solid_end - solid_start
			} else {
				solid_end + arc_degrees - solid_start
			};
			push_solid_sweep(&mut partitions, center_xz, ring_scale, solid_start, sweep);
		}
	} else {
		let first = portals[0].t * arc_degrees;
		let last = portals[portals.len() - 1].t * arc_degrees;
		push_solid_sweep(
			&mut partitions,
			center_xz,
			ring_scale,
			0.0,
			(first - OPEN_HALF_DEG).max(0.0),
		);
		for i in 0..portals.len().saturating_sub(1) {
			let c0 = portals[i].t * arc_degrees;
			let c1 = portals[i + 1].t * arc_degrees;
			let solid_start = c0 + OPEN_HALF_DEG;
			let solid_end = c1 - OPEN_HALF_DEG;
			push_solid_sweep(
				&mut partitions,
				center_xz,
				ring_scale,
				solid_start,
				(solid_end - solid_start).max(0.0),
			);
		}
		push_solid_sweep(
			&mut partitions,
			center_xz,
			ring_scale,
			last + OPEN_HALF_DEG,
			(arc_degrees - (last + OPEN_HALF_DEG)).max(0.0),
		);
	}

	partitions
}

fn push_solid_sweep(
	partitions: &mut Vec<PartitionNode>,
	center_xz: Vec3,
	ring_scale: Vec3,
	start_deg: f32,
	sweep_deg: f32,
) {
	if sweep_deg > 1e-2 {
		partitions.push(PartitionNode::rough_stone(
			Partition::arc(sweep_deg),
			Placement::new(center_xz, start_deg.to_radians()).with_scale(ring_scale),
		));
	}
}

fn norm_deg(deg: f32) -> f32 {
	let mut d = deg % 360.0;
	if d < 0.0 {
		d += 360.0;
	}
	d
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::walling::portal::regions_overlap;

	fn cardinal_must_assign() -> Vec<MustAssignPortal> {
		vec![
			MustAssignPortal::at(0.0, Portal::Door),
			MustAssignPortal::at(0.25, Portal::Window),
			MustAssignPortal::at(0.5, Portal::Window),
			MustAssignPortal::at(0.75, Portal::Window),
		]
	}

	fn closed_arc(optional: (u32, u32), seed: i32) -> ArcWall {
		ArcWall::new(ArcWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			arc_degrees: 360.0,
			must_assign: cardinal_must_assign(),
			must_not_assign: vec![],
			portal_noise: NoiseParams { seed, ..NoiseParams::default() },
			optional_portals: optional,
		})
	}

	fn arc_foot(arc_degrees: f32) -> PortalFootprint {
		let half_t = OPEN_HALF_DEG / arc_degrees.max(SEG_DEG);
		PortalFootprint { half_t, closed: is_closed(arc_degrees) }
	}

	#[test]
	fn must_assign_cardinals_without_optional() -> anyhow::Result<()> {
		let wall = closed_arc((0, 0), 1);
		assert_eq!(wall.portals.len(), 4);
		assert!(matches!(wall.portals[0].portal, Portal::Door));
		assert!((wall.portals[0].t - 0.0).abs() < 1e-5);
		assert!((wall.portals[1].t - 0.25).abs() < 1e-5);
		assert!((wall.arc_degrees - 360.0).abs() < 1e-3);
		let slices = wall
			.partitions
			.iter()
			.filter(|w| matches!(w.geometry, Partition::SliceArc(_)))
			.count();
		assert_eq!(slices, 14);
		Ok(())
	}

	#[test]
	fn optional_portals_stay_in_can_assign() -> anyhow::Result<()> {
		let wall = closed_arc((0, 4), 42);
		assert!(wall.portals.len() >= 4);
		assert!(wall.portals.len() <= 8);
		let foot = arc_foot(wall.arc_degrees);
		for i in 0..wall.portals.len() {
			for j in (i + 1)..wall.portals.len() {
				let a = foot.interval(wall.portals[i].t);
				let b = foot.interval(wall.portals[j].t);
				assert!(
					!regions_overlap(a, b, true),
					"portals {} and {} overlap",
					wall.portals[i].t,
					wall.portals[j].t
				);
			}
		}
		Ok(())
	}

	#[test]
	fn must_not_blocks_optional() -> anyhow::Result<()> {
		let wall = ArcWall::new(ArcWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			arc_degrees: 360.0,
			must_assign: vec![MustAssignPortal::at(0.0, Portal::Door)],
			must_not_assign: vec![WallRegion::span(0.1, 0.9)],
			portal_noise: NoiseParams { seed: 7, ..NoiseParams::default() },
			optional_portals: (4, 4),
		});
		assert_eq!(wall.portals.len(), 1);
		Ok(())
	}

	#[test]
	fn open_half_arc_has_no_wrap_solid() -> anyhow::Result<()> {
		let wall = ArcWall::new(ArcWallParams {
			center_xz: Vec3::ZERO,
			radius: 4.0,
			storey_height: 3.0,
			arc_degrees: 180.0,
			must_assign: vec![
				MustAssignPortal::at(0.25, Portal::Window),
				MustAssignPortal::at(0.75, Portal::Window),
			],
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
		});
		assert!((wall.arc_degrees - 180.0).abs() < 1e-3);
		assert_eq!(wall.portals.len(), 2);
		let solids = wall
			.partitions
			.iter()
			.filter(|w| matches!(w.geometry, Partition::Arc(_)))
			.count();
		assert_eq!(solids, 3);
		Ok(())
	}
}
