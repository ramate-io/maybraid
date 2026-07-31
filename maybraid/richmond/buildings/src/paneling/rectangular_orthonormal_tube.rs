//! Rectangular orthonormal tube from a polyline of rectangular cross-sections.
//!
//! Each station has a full width (centered on `bottom_middle`) and a height.
//! A single tube-level roll banks every station in its average perpendicular
//! plane. The four corner polylines become floor, ceiling, left, and right
//! [`ClippedRectangularStrip`] faces.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::path_frame::{self, TubeFrame};
use crate::paneling::rect_fit::RectInset;
use crate::paneling::tube::TubeCorners;

/// One authored station along a [`RectangularOrthonormalTube`] centerline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectangularOrthonormalTubeNode {
	pub bottom_middle: Vec3,
	/// Full width, centered on [`Self::bottom_middle`].
	pub width: f32,
	pub height: f32,
}

impl RectangularOrthonormalTubeNode {
	pub fn new(bottom_middle: Vec3, width: f32, height: f32) -> Self {
		Self {
			bottom_middle,
			width,
			height,
		}
	}
}

/// Polyline tube: four [`ClippedRectangularStrip`] faces from rectangular stations.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularOrthonormalTube {
	style: PanelStyle,
	joint_policy: PanelComplexJointPolicy,
	roll: f32,
	nodes: Vec<RectangularOrthonormalTubeNode>,
	floor: ClippedRectangularStrip,
	ceiling: ClippedRectangularStrip,
	left: ClippedRectangularStrip,
	right: ClippedRectangularStrip,
}

impl RectangularOrthonormalTube {
	pub fn new(style: PanelStyle) -> Self {
		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			roll: 0.0,
			nodes: Vec::new(),
			floor: ClippedRectangularStrip::new(style),
			ceiling: ClippedRectangularStrip::new(style),
			left: ClippedRectangularStrip::new(style),
			right: ClippedRectangularStrip::new(style),
		}
	}

	pub fn rough_stone() -> Self {
		Self::new(PanelStyle::RoughStonework)
	}

	pub fn shepherds_thatch() -> Self {
		Self::new(PanelStyle::ShepherdsThatch)
	}

	/// Bulk construct with solid faces (no bay insets).
	pub fn from_nodes(
		style: PanelStyle,
		nodes: impl IntoIterator<Item = RectangularOrthonormalTubeNode>,
		roll: f32,
	) -> Self {
		Self::from_nodes_with_insets(
			style,
			nodes,
			roll,
			std::iter::empty(),
			std::iter::empty(),
			std::iter::empty(),
			std::iter::empty(),
		)
	}

	/// Bulk construct. Per-face inset lists should match bay count (`nodes - 1`);
	/// empty or mismatched lengths pad/truncate with `debug_assert` (same as strips).
	pub fn from_nodes_with_insets(
		style: PanelStyle,
		nodes: impl IntoIterator<Item = RectangularOrthonormalTubeNode>,
		roll: f32,
		floor_insets: impl IntoIterator<Item = Option<RectInset>>,
		ceiling_insets: impl IntoIterator<Item = Option<RectInset>>,
		left_insets: impl IntoIterator<Item = Option<RectInset>>,
		right_insets: impl IntoIterator<Item = Option<RectInset>>,
	) -> Self {
		let nodes: Vec<RectangularOrthonormalTubeNode> = nodes.into_iter().collect();
		if nodes.len() < 2 {
			debug_assert!(
				false,
				"RectangularOrthonormalTube::from_nodes requires at least 2 stations (got {})",
				nodes.len()
			);
			return Self::new(style);
		}

		let positions: Vec<Vec3> = nodes.iter().map(|n| n.bottom_middle).collect();
		let mut bottom_left = Vec::with_capacity(nodes.len());
		let mut bottom_right = Vec::with_capacity(nodes.len());
		let mut top_left = Vec::with_capacity(nodes.len());
		let mut top_right = Vec::with_capacity(nodes.len());
		for i in 0..nodes.len() {
			let c = corners_at_framed(&nodes, &positions, i, roll);
			bottom_left.push(c.bottom_left);
			bottom_right.push(c.bottom_right);
			top_left.push(c.top_left);
			top_right.push(c.top_right);
		}

		let bay_count = nodes.len() - 1;
		let floor_insets = normalize_insets(floor_insets, bay_count, "floor");
		let ceiling_insets = normalize_insets(ceiling_insets, bay_count, "ceiling");
		let left_insets = normalize_insets(left_insets, bay_count, "left");
		let right_insets = normalize_insets(right_insets, bay_count, "right");

		let floor = ClippedRectangularStrip::from_lines(
			style,
			bottom_left.clone(),
			bottom_right.clone(),
			floor_insets,
		);
		let ceiling = ClippedRectangularStrip::from_lines(
			style,
			top_left.clone(),
			top_right.clone(),
			ceiling_insets,
		);
		let left = ClippedRectangularStrip::from_lines(
			style,
			bottom_left.clone(),
			top_left.clone(),
			left_insets,
		);
		let right =
			ClippedRectangularStrip::from_lines(style, bottom_right, top_right, right_insets);

		Self {
			style,
			joint_policy: PanelComplexJointPolicy::default(),
			roll,
			nodes,
			floor,
			ceiling,
			left,
			right,
		}
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self.floor = std::mem::replace(&mut self.floor, ClippedRectangularStrip::new(self.style))
			.with_joint_policy(joint_policy);
		self.ceiling =
			std::mem::replace(&mut self.ceiling, ClippedRectangularStrip::new(self.style))
				.with_joint_policy(joint_policy);
		self.left = std::mem::replace(&mut self.left, ClippedRectangularStrip::new(self.style))
			.with_joint_policy(joint_policy);
		self.right = std::mem::replace(&mut self.right, ClippedRectangularStrip::new(self.style))
			.with_joint_policy(joint_policy);
		self
	}

	pub fn nodes(&self) -> &[RectangularOrthonormalTubeNode] {
		&self.nodes
	}

	pub fn roll(&self) -> f32 {
		self.roll
	}

	pub fn floor(&self) -> &ClippedRectangularStrip {
		&self.floor
	}

	pub fn ceiling(&self) -> &ClippedRectangularStrip {
		&self.ceiling
	}

	pub fn left(&self) -> &ClippedRectangularStrip {
		&self.left
	}

	pub fn right(&self) -> &ClippedRectangularStrip {
		&self.right
	}

	/// Corner rails for every station (same order as [`Self::nodes`]).
	pub fn corners(&self) -> Vec<TubeCorners> {
		let positions: Vec<Vec3> = self.nodes.iter().map(|n| n.bottom_middle).collect();
		(0..self.nodes.len())
			.map(|i| corners_at_framed(&self.nodes, &positions, i, self.roll))
			.collect()
	}

	pub fn frame_at(&self, index: usize) -> Option<TubeFrame> {
		if index >= self.nodes.len() {
			return None;
		}
		Some(frame_at(&self.nodes, index, self.roll))
	}
}

impl BuildingComponents for RectangularOrthonormalTube {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		out.extend(self.floor.panel_nodes_for_level(level));
		out.extend(self.ceiling.panel_nodes_for_level(level));
		out.extend(self.left.panel_nodes_for_level(level));
		out.extend(self.right.panel_nodes_for_level(level));
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		out.extend(self.floor.joint_nodes_for_level(level));
		out.extend(self.ceiling.joint_nodes_for_level(level));
		out.extend(self.left.joint_nodes_for_level(level));
		out.extend(self.right.joint_nodes_for_level(level));
		out
	}
}

fn normalize_insets(
	insets: impl IntoIterator<Item = Option<RectInset>>,
	bay_count: usize,
	face: &str,
) -> Vec<Option<RectInset>> {
	let mut insets: Vec<Option<RectInset>> = insets.into_iter().collect();
	if insets.is_empty() {
		return vec![None; bay_count];
	}
	if insets.len() != bay_count {
		debug_assert!(
			false,
			"RectangularOrthonormalTube {} insets.len()={} != bay_count={}",
			face,
			insets.len(),
			bay_count
		);
		insets.resize_with(bay_count, || None);
	}
	insets
}

/// Average tangent and zero-roll basis, then apply tube roll about the tangent.
pub fn frame_at(
	nodes: &[RectangularOrthonormalTubeNode],
	index: usize,
	roll: f32,
) -> TubeFrame {
	let positions: Vec<Vec3> = nodes.iter().map(|n| n.bottom_middle).collect();
	path_frame::path_frame(&positions, index, roll)
}

pub fn corners_at(
	nodes: &[RectangularOrthonormalTubeNode],
	index: usize,
	roll: f32,
) -> TubeCorners {
	let positions: Vec<Vec3> = nodes.iter().map(|n| n.bottom_middle).collect();
	corners_at_framed(nodes, &positions, index, roll)
}

fn corners_at_framed(
	nodes: &[RectangularOrthonormalTubeNode],
	positions: &[Vec3],
	index: usize,
	roll: f32,
) -> TubeCorners {
	let node = &nodes[index];
	let frame = path_frame::path_frame(positions, index, roll);
	let half = node.width * 0.5;
	let top_middle = node.bottom_middle + frame.up * node.height;
	TubeCorners {
		bottom_left: node.bottom_middle - frame.right * half,
		bottom_right: node.bottom_middle + frame.right * half,
		top_left: top_middle - frame.right * half,
		top_right: top_middle + frame.right * half,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::paneling::clipped_rectangular_strip::ClippedRectangularStripPiece;
	use richmond_building_components::panels::PanelGeometry;

	fn approx_eq(a: Vec3, b: Vec3) -> bool {
		(a - b).length() < 1e-4
	}

	fn level_node(z: f32, width: f32, height: f32) -> RectangularOrthonormalTubeNode {
		RectangularOrthonormalTubeNode::new(Vec3::new(0.0, 0.0, z), width, height)
	}

	#[test]
	fn level_straight_cardinal_corners() {
		let nodes = vec![
			level_node(0.0, 2.0, 2.0),
			level_node(2.0, 2.0, 2.0),
			level_node(4.0, 2.0, 2.0),
		];
		let tube = RectangularOrthonormalTube::from_nodes(PanelStyle::RoughStonework, nodes, 0.0);
		assert_eq!(tube.nodes().len(), 3);

		let c0 = tube.corners()[0];
		assert!(approx_eq(c0.bottom_left, Vec3::new(-1.0, 0.0, 0.0)));
		assert!(approx_eq(c0.bottom_right, Vec3::new(1.0, 0.0, 0.0)));
		assert!(approx_eq(c0.top_left, Vec3::new(-1.0, 2.0, 0.0)));
		assert!(approx_eq(c0.top_right, Vec3::new(1.0, 2.0, 0.0)));

		assert_eq!(tube.floor().pieces().len(), 2);
		assert!(matches!(
			tube.floor().pieces()[0],
			ClippedRectangularStripPiece::Solid(_)
		));
		assert!(tube
			.floor()
			.pieces()
			.iter()
			.flat_map(|p| p.panels())
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
		assert_eq!(tube.ceiling().pieces().len(), 2);
		assert_eq!(tube.left().pieces().len(), 2);
		assert_eq!(tube.right().pieces().len(), 2);
	}

	#[test]
	fn pitched_path_tilts_up_with_average_tangent() {
		let nodes = vec![
			RectangularOrthonormalTubeNode::new(Vec3::new(0.0, 0.0, 0.0), 2.0, 1.0),
			RectangularOrthonormalTubeNode::new(Vec3::new(0.0, 1.0, 2.0), 2.0, 1.0),
			RectangularOrthonormalTubeNode::new(Vec3::new(0.0, 2.0, 4.0), 2.0, 1.0),
		];
		let frame = frame_at(&nodes, 1, 0.0);
		assert!(
			frame.up.z.abs() > 0.1,
			"up should tilt with pitch, got {:?}",
			frame.up
		);
		assert!(
			frame.up.x.abs() < 1e-4,
			"pitch in YZ should keep up.x ~ 0, got {:?}",
			frame.up
		);
		assert!(approx_eq(frame.right, Vec3::X) || approx_eq(frame.right, Vec3::new(1.0, 0.0, 0.0)));
		assert!(frame.right.y.abs() < 1e-4);
	}

	#[test]
	fn non_zero_roll_banks_all_stations() {
		let nodes = vec![
			level_node(0.0, 2.0, 2.0),
			level_node(2.0, 2.0, 2.0),
			level_node(4.0, 2.0, 2.0),
		];
		let roll = std::f32::consts::FRAC_PI_2;
		let frame = frame_at(&nodes, 1, roll);
		// +90° about +Z: right₀=X → Y, up₀=Y → -X
		assert!(approx_eq(frame.right, Vec3::Y));
		assert!(approx_eq(frame.up, Vec3::new(-1.0, 0.0, 0.0)));

		let c = corners_at(&nodes, 1, roll);
		assert!(approx_eq(c.bottom_left, Vec3::new(0.0, -1.0, 2.0)));
		assert!(approx_eq(c.bottom_right, Vec3::new(0.0, 1.0, 2.0)));

		// Same roll at every station.
		let frame0 = frame_at(&nodes, 0, roll);
		assert!(approx_eq(frame0.right, Vec3::Y));
		assert!(approx_eq(frame0.up, Vec3::new(-1.0, 0.0, 0.0)));
	}

	#[test]
	fn middle_bay_left_inset_clips() {
		let nodes = vec![
			level_node(0.0, 2.0, 2.0),
			level_node(2.0, 2.0, 2.0),
			level_node(4.0, 2.0, 2.0),
			level_node(6.0, 2.0, 2.0),
		];
		let tube = RectangularOrthonormalTube::from_nodes_with_insets(
			PanelStyle::RoughStonework,
			nodes,
			0.0,
			[None, None, None],
			[None, None, None],
			[None, Some(RectInset::uniform(0.35)), None],
			[None, None, None],
		);
		assert_eq!(tube.left().pieces().len(), 3);
		assert!(matches!(
			tube.left().pieces()[0],
			ClippedRectangularStripPiece::Solid(_)
		));
		assert!(matches!(
			tube.left().pieces()[1],
			ClippedRectangularStripPiece::Clipped(_)
		));
		assert!(matches!(
			tube.left().pieces()[2],
			ClippedRectangularStripPiece::Solid(_)
		));
		let clipped = match &tube.left().pieces()[1] {
			ClippedRectangularStripPiece::Clipped(c) => c,
			_ => unreachable!(),
		};
		assert_eq!(clipped.panels().len(), 4);
	}

	#[test]
	#[should_panic(expected = "at least 2 stations")]
	fn short_input_debug_asserts() {
		let _ = RectangularOrthonormalTube::from_nodes(
			PanelStyle::RoughStonework,
			[level_node(0.0, 2.0, 1.0)],
			0.0,
		);
	}

	#[test]
	fn two_stations_minimum_builds_faces() {
		let tube = RectangularOrthonormalTube::from_nodes(
			PanelStyle::RoughStonework,
			[level_node(0.0, 2.0, 1.0), level_node(2.0, 2.0, 1.0)],
			0.0,
		);
		assert_eq!(tube.nodes().len(), 2);
		assert_eq!(tube.floor().pieces().len(), 1);
		assert!(matches!(
			tube.floor().pieces()[0],
			ClippedRectangularStripPiece::Solid(_)
		));
	}
}
