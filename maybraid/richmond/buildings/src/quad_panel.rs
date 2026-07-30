//! Ruled quad between two lines: two [`TessellatedTrianglePanel`]s + optional crease joint.
//!
//! Line A \((a_0 \to a_1)\) and line B \((b_0 \to b_1)\) form triangles
//! \((a_0, a_1, b_1)\) and \((a_0, b_1, b_0)\) sharing diagonal \(a_0\)–\(b_1\).
//! A [`JointNode`] is emitted when the dihedral kink between the triangles meets
//! [`QuadPanelJointPolicy`].

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::{JointNode, JointPost};
use richmond_building_components::panels::{yaw_along_xz, PanelNode, PanelStyle, DEFAULT_MIN_JOINT_ANGLE};
use richmond_building_components::BuildingComponents;

use crate::tessellated_triangle_panel::TessellatedTrianglePanel;

/// When to spawn a crease joint from the dihedral kink between the two triangles.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadPanelJointPolicy {
	/// Spawn a joint when the dihedral kink (radians) is ≥ this threshold.
	pub min_dihedral_rad: f32,
}

impl Default for QuadPanelJointPolicy {
	fn default() -> Self {
		Self { min_dihedral_rad: DEFAULT_MIN_JOINT_ANGLE }
	}
}

impl QuadPanelJointPolicy {
	pub fn always() -> Self {
		Self { min_dihedral_rad: 0.0 }
	}

	pub fn never() -> Self {
		Self { min_dihedral_rad: f32::INFINITY }
	}

	pub fn min_dihedral_rad(min_dihedral_rad: f32) -> Self {
		Self { min_dihedral_rad: min_dihedral_rad.max(0.0) }
	}
}

/// Two lines filled with tessellated panel triangles and an optional crease joint.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadPanel {
	pub style: PanelStyle,
	pub a0: Vec3,
	pub a1: Vec3,
	pub b0: Vec3,
	pub b1: Vec3,
	pub joint_policy: QuadPanelJointPolicy,
}

impl QuadPanel {
	pub fn new(
		style: PanelStyle,
		a0: Vec3,
		a1: Vec3,
		b0: Vec3,
		b1: Vec3,
		joint_policy: QuadPanelJointPolicy,
	) -> Self {
		Self { style, a0, a1, b0, b1, joint_policy }
	}

	pub fn rough_stone(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Self {
		Self::new(PanelStyle::RoughStonework, a0, a1, b0, b1, QuadPanelJointPolicy::default())
	}

	pub fn shepherds_thatch(a0: Vec3, a1: Vec3, b0: Vec3, b1: Vec3) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a0, a1, b0, b1, QuadPanelJointPolicy::default())
	}

	pub fn with_joint_policy(mut self, joint_policy: QuadPanelJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self
	}

	/// Triangle on line A toward \(b_1\): \((a_0, a_1, b_1)\).
	pub fn triangle_a(&self) -> TessellatedTrianglePanel {
		TessellatedTrianglePanel::new(self.style, self.a0, self.a1, self.b1)
	}

	/// Triangle on line B toward \(a_0\): \((a_0, b_1, b_0)\).
	pub fn triangle_b(&self) -> TessellatedTrianglePanel {
		TessellatedTrianglePanel::new(self.style, self.a0, self.b1, self.b0)
	}

	/// Unit normals of the two triangles, or [`None`] if either is degenerate.
	pub fn triangle_normals(&self) -> Option<(Vec3, Vec3)> {
		let n0 = triangle_normal(self.a0, self.a1, self.b1)?;
		let n1 = triangle_normal(self.a0, self.b1, self.b0)?;
		Some((n0, n1))
	}

	/// Dihedral kink (radians) between the two triangle normals.
	///
	/// \(0\) when coplanar with matching orientation; grows toward \(\pi\) as the fold opens.
	pub fn dihedral_kink(&self) -> Option<f32> {
		let (n0, n1) = self.triangle_normals()?;
		Some(n0.dot(n1).clamp(-1.0, 1.0).acos())
	}

	/// Crease [`JointNode`] when the policy threshold is met, else [`None`].
	pub fn joint_node(&self) -> Option<JointNode> {
		let kink = self.dihedral_kink()?;
		if kink < self.joint_policy.min_dihedral_rad {
			return None;
		}
		let diag = self.b1 - self.a0;
		let len = diag.length();
		if len < 1e-8 {
			return None;
		}
		let mid = (self.a0 + self.b1) * 0.5;
		let yaw = yaw_along_xz(diag.x, diag.z);
		let placement = JointPost::placed_along_edge(mid, yaw, kink_for_scale(kink), len);
		Some(JointNode::rough_stone_post(placement))
	}
}

fn kink_for_scale(dihedral_kink: f32) -> f32 {
	// Size from crease fold; treat coplanar-opposite (≈π) like a sharp fold via min(k, π−k).
	dihedral_kink.min((std::f32::consts::PI - dihedral_kink).abs())
}

fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Option<Vec3> {
	let n = (b - a).cross(c - a);
	let len = n.length();
	if len < 1e-12 {
		None
	} else {
		Some(n / len)
	}
}

impl BuildingComponents for QuadPanel {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Vec<PanelNode> {
		let mut out = self.triangle_a().panel_nodes_for_level(level);
		out.extend(self.triangle_b().panel_nodes_for_level(level));
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Vec<JointNode> {
		self.joint_node().into_iter().collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use richmond_building_components::BuildingComponents;

	#[test]
	fn coplanar_emits_two_panels_and_no_joint() {
		// Both triangles in the XZ plane (y = 0).
		let quad = QuadPanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 2.0),
			Vec3::new(2.0, 0.0, 2.0),
		);
		assert_eq!(quad.panel_nodes_for_level(LodSceneLevel::High).len(), 2);
		let kink = quad.dihedral_kink().expect("kink");
		assert!(kink < 1e-3, "expected near-coplanar, got {kink}");
		assert!(quad.joint_node().is_none());
	}

	#[test]
	fn folded_emits_joint() {
		// XZ triangle + YZ triangle share a0–b1; normals ⊥ → 90° dihedral.
		let quad = QuadPanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(0.0, 1.0, 0.0),
			Vec3::new(0.0, 0.0, 1.0),
		)
		.with_joint_policy(QuadPanelJointPolicy::default());
		let kink = quad.dihedral_kink().expect("kink");
		assert!(
			(kink - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
			"expected ~90° fold, got {kink}"
		);
		assert!(quad.joint_node().is_some());
		assert_eq!(quad.panel_nodes_for_level(LodSceneLevel::High).len(), 2);
	}

	#[test]
	fn never_policy_suppresses_joint() {
		let quad = QuadPanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(0.0, 1.0, 0.0),
			Vec3::new(0.0, 0.0, 1.0),
		)
		.with_joint_policy(QuadPanelJointPolicy::never());
		assert!(quad.joint_node().is_none());
	}
}
