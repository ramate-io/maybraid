//! One-kink hall connecting two oriented openings via a [`Tube`].
//!
//! Each end is a [`HallOpening`] (wrapper over [`MappedOpening`]: outward quad +
//! XZ facing). Rays along those orientations meet in plan; the junction height
//! and cross-section are length-weighted lerps of the two ends.
//!
//! Prefer preparing ends with [`MappedOpening::widened`] (jamb overrun) and
//! [`MappedOpening::raised`] (header clearance) before constructing the hall.

use std::ops::Deref;

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::connecting::geom::{
	lerp_tube_nodes, normalize_xz, opening_to_tube_node, plan_kink, EPS,
};
use crate::openings::MappedOpening;
use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::tube::{Tube, TubeCrossSectionNode, TubeFaces};

/// Horizontal connector opening: same contact as [`MappedOpening`], typed for
/// [`ConnectingHall`] so it cannot be confused with a stairwell end.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HallOpening(MappedOpening);

impl HallOpening {
	pub fn new(mapped: MappedOpening) -> Self {
		Self(mapped)
	}

	pub fn mapped(self) -> MappedOpening {
		self.0
	}
}

impl From<MappedOpening> for HallOpening {
	fn from(mapped: MappedOpening) -> Self {
		Self(mapped)
	}
}

impl Deref for HallOpening {
	type Target = MappedOpening;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

/// Small connector: two openings → one-kink plan path → [`Tube`].
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectingHall {
	style: PanelStyle,
	end_a: HallOpening,
	end_b: HallOpening,
	midpoint: Vec3,
	stations: [TubeCrossSectionNode; 3],
	tube: Tube,
}

impl ConnectingHall {
	pub fn new(
		style: PanelStyle,
		end_a: impl Into<HallOpening>,
		end_b: impl Into<HallOpening>,
	) -> Self {
		let end_a = end_a.into();
		let end_b = end_b.into();
		match build_stations(end_a, end_b) {
			Some((midpoint, stations)) => {
				let tube = Tube::from_nodes(style, stations);
				Self { style, end_a, end_b, midpoint, stations, tube }
			}
			None => {
				debug_assert!(false, "ConnectingHall: orientation rays do not meet in plan");
				Self {
					style,
					end_a,
					end_b,
					midpoint: Vec3::ZERO,
					stations: [TubeCrossSectionNode::new(Vec3::ZERO, 0.0, 0.0, 0.0, 0.0, 0.0); 3],
					tube: Tube::new(style),
				}
			}
		}
	}

	pub fn rough_stone(end_a: impl Into<HallOpening>, end_b: impl Into<HallOpening>) -> Self {
		Self::new(PanelStyle::RoughStonework, end_a, end_b)
	}

	pub fn with_faces(mut self, faces: TubeFaces) -> Self {
		self.tube = std::mem::replace(&mut self.tube, Tube::new(self.style)).with_faces(faces);
		self
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.tube = std::mem::replace(&mut self.tube, Tube::new(self.style))
			.with_joint_policy(joint_policy);
		self
	}

	pub fn tube(&self) -> &Tube {
		&self.tube
	}

	pub fn midpoint(&self) -> Vec3 {
		self.midpoint
	}

	pub fn endpoints(&self) -> (HallOpening, HallOpening) {
		(self.end_a, self.end_b)
	}

	pub fn stations(&self) -> &[TubeCrossSectionNode; 3] {
		&self.stations
	}
}

impl BuildingComponents for ConnectingHall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.tube.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.tube.joint_nodes_for_level(level)
	}
}

fn build_stations(
	end_a: HallOpening,
	end_b: HallOpening,
) -> Option<(Vec3, [TubeCrossSectionNode; 3])> {
	let node_a = opening_to_tube_node(end_a.mapped())?;
	let node_b = opening_to_tube_node(end_b.mapped())?;

	let p_a = Vec2::new(node_a.bottom_middle.x, node_a.bottom_middle.z);
	let p_b = Vec2::new(node_b.bottom_middle.x, node_b.bottom_middle.z);
	let d_a = normalize_xz(end_a.orientation);
	let d_b = normalize_xz(end_b.orientation);

	// Prefer the plan kink where the opening rays meet. When openings are skewed
	// (e.g. an arc-door chord offset from the facing axis) the forward rays may
	// miss — fall back to the midpoint between the openings.
	let m_xz = plan_kink(p_a, d_a, p_b, d_b);

	let l_a = (m_xz - p_a).length().max(EPS);
	let l_b = (m_xz - p_b).length().max(EPS);
	let inv = 1.0 / (l_a + l_b);
	let w_a = l_b * inv;
	let w_b = l_a * inv;

	let h_m = w_a * node_a.bottom_middle.y + w_b * node_b.bottom_middle.y;
	let mid = Vec3::new(m_xz.x, h_m, m_xz.y);
	let node_mid = lerp_tube_nodes(node_a, node_b, w_a, w_b, mid);

	Some((mid, [node_a, node_mid, node_b]))
}

#[cfg(test)]
mod tests {
	use super::*;

	fn opening_facing(
		center: Vec3,
		half_w: f32,
		half_h: f32,
		orient: Vec2,
	) -> anyhow::Result<MappedOpening> {
		let d = normalize_xz(orient)
			.ok_or_else(|| anyhow::anyhow!("orientation too short: {orient:?}"))?;
		let right = Vec3::new(-d.y, 0.0, d.x);
		let up = Vec3::Y;
		let bl = center - right * half_w;
		let br = center + right * half_w;
		let tl = bl + up * (half_h * 2.0);
		let tr = br + up * (half_h * 2.0);
		Ok(MappedOpening::from_corners(bl, br, tl, tr, orient))
	}

	#[test]
	fn opposite_openings_meet_on_bisector() -> anyhow::Result<()> {
		// A at x=-4 facing +X; B at x=+4 facing -X → mid at origin.
		let a = opening_facing(Vec3::new(-4.0, 0.0, 0.0), 1.0, 1.0, Vec2::X)?;
		let b = opening_facing(Vec3::new(4.0, 0.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.midpoint();
		assert!(mid.x.abs() < 1e-3, "mid.x={:?}", mid.x);
		assert!(mid.z.abs() < 1e-3, "mid.z={:?}", mid.z);
		assert!(mid.y.abs() < 1e-3);
		assert_eq!(hall.tube().nodes().len(), 3);
		assert!(!hall.tube().floor().pieces().is_empty());
		Ok(())
	}

	#[test]
	fn height_is_length_weighted() -> anyhow::Result<()> {
		// Kinked: A at z=-1 facing +Z, B at x=4 facing -X → mid at origin.
		// L_a=1, L_b=4 → h = (4*0 + 1*4)/(1+4) = 0.8
		let a = opening_facing(Vec3::new(0.0, 0.0, -1.0), 1.0, 1.0, Vec2::Y)?;
		let b = opening_facing(Vec3::new(4.0, 4.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.midpoint();
		assert!(mid.x.abs() < 1e-3 && mid.z.abs() < 1e-3, "mid={mid:?}");
		assert!((mid.y - 0.8).abs() < 1e-3, "mid.y={}", mid.y);
		Ok(())
	}

	#[test]
	fn kinked_orientations_intersect() -> anyhow::Result<()> {
		let a = opening_facing(Vec3::new(0.0, 0.0, -3.0), 1.0, 1.0, Vec2::Y)?;
		let b = opening_facing(Vec3::new(3.0, 0.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.midpoint();
		assert!((mid.x - 0.0).abs() < 1e-3);
		assert!((mid.z - 0.0).abs() < 1e-3);
		assert_eq!(hall.stations()[1].bottom_middle, mid);
		Ok(())
	}

	#[test]
	fn parallel_orientations_fall_back_to_midpoint() -> anyhow::Result<()> {
		// Same-direction parallel rays miss; connector falls back to the plan midpoint.
		let a = opening_facing(Vec3::new(0.0, 0.0, 0.0), 1.0, 1.0, Vec2::X)?;
		let b = opening_facing(Vec3::new(0.0, 0.0, 2.0), 1.0, 1.0, Vec2::X)?;
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.midpoint();
		assert!((mid.x - 0.0).abs() < 1e-3);
		assert!((mid.z - 1.0).abs() < 1e-3);
		assert_eq!(hall.tube().nodes().len(), 3);
		Ok(())
	}

	#[test]
	fn raised_ends_lift_hall_tops() -> anyhow::Result<()> {
		let a = opening_facing(Vec3::new(-4.0, 0.0, 0.0), 1.0, 1.0, Vec2::X)?;
		let b = opening_facing(Vec3::new(4.0, 0.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let (.., tl, _) = a.endpoint_corners();
		let lintel_y = tl.y;
		let hall = ConnectingHall::rough_stone(a.raised(0.3), b.raised(0.3));
		let top = hall.stations()[0]
			.top_middle
			.ok_or_else(|| anyhow::anyhow!("top_middle missing"))?;
		assert!((top.y - (lintel_y + 0.3)).abs() < 1e-3, "top.y={} lintel={}", top.y, lintel_y);
		assert!((hall.stations()[0].height - 2.3).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn unequal_plumb_openings_keep_plumb_kink() -> anyhow::Result<()> {
		// Same geometry as `height_is_length_weighted`: world-lerping lintels used
		// to put the mid top at a plan offset even though both faces are vertical.
		let a = opening_facing(Vec3::new(0.0, 0.0, -1.0), 1.0, 1.0, Vec2::Y)?;
		let b = opening_facing(Vec3::new(4.0, 4.0, 0.0), 1.0, 1.0, -Vec2::X)?;
		let hall = ConnectingHall::rough_stone(a, b);
		let mid = hall.stations()[1];
		let top = mid.top_middle.ok_or_else(|| anyhow::anyhow!("mid top_middle"))?;
		assert!((top.x - mid.bottom_middle.x).abs() < 1e-3);
		assert!((top.z - mid.bottom_middle.z).abs() < 1e-3);
		assert!((top.y - mid.bottom_middle.y - mid.height).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn pitched_end_does_not_lean_the_kink() -> anyhow::Result<()> {
		let a = opening_facing(Vec3::new(0.0, 0.0, -3.0), 1.0, 1.0, Vec2::Y)?;
		let (bl, br, tl, tr) = opening_facing(Vec3::new(3.0, 0.0, 0.0), 1.0, 1.0, -Vec2::X)?
			.endpoint_corners();
		// Inset the lintel along +X so the B face pitches like a trazaloid.
		let b = MappedOpening::from_corners(
			bl,
			br,
			tl + Vec3::X * 0.4,
			tr + Vec3::X * 0.4,
			-Vec2::X,
		);
		let hall = ConnectingHall::rough_stone(a, b);
		let end_b = hall.stations()[2];
		let end_top = end_b.top_middle.ok_or_else(|| anyhow::anyhow!("end top"))?;
		assert!(
			(end_top.x - end_b.bottom_middle.x).abs() > 0.3,
			"end should keep host-face pitch, got {end_top:?}"
		);
		let mid = hall.stations()[1];
		let mid_top = mid.top_middle.ok_or_else(|| anyhow::anyhow!("mid top"))?;
		assert!((mid_top.x - mid.bottom_middle.x).abs() < 1e-3);
		assert!((mid_top.z - mid.bottom_middle.z).abs() < 1e-3);
		Ok(())
	}
}
