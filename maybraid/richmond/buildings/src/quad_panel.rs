//! Ruled quad between two lines: two [`TessellatedTrianglePanel`]s + optional crease joint.
//!
//! Line A \((a_0 \to a_1)\) and line B \((b_0 \to b_1)\) form triangles
//! \((a_0, a_1, b_1)\) and \((a_0, b_1, b_0)\) sharing diagonal \(a_0\)–\(b_1\).
//! A [`JointNode`] is emitted when the dihedral kink between the triangles meets
//! [`QuadPanelJointPolicy`].
//!
//! Each corner carries a panel thickness. Line A/B thicknesses are averages of their
//! endpoints; both triangles share that pair-average as their thickness. The crease
//! joint uses that shared thickness for kit \(X/Z\) scale and aligns kit \(+Y\) with
//! the diagonal.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::{JointNode, JointPost};
use richmond_building_components::panels::{PanelNode, PanelStyle, DEFAULT_MIN_JOINT_ANGLE};
use richmond_building_components::BuildingComponents;

use crate::tessellated_triangle_panel::TessellatedTrianglePanel;

/// Default world thickness matching unscaled panel kits (\(Y \in [-0.2, 0.2]\)).
pub const DEFAULT_PANEL_THICKNESS: f32 = 0.4;

/// World position + panel thickness at a quad corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadCorner {
	pub position: Vec3,
	pub thickness: f32,
}

impl QuadCorner {
	pub fn new(position: Vec3, thickness: f32) -> Self {
		Self { position, thickness: thickness.max(1e-4) }
	}

	/// Corner with [`DEFAULT_PANEL_THICKNESS`].
	pub fn at(position: Vec3) -> Self {
		Self::new(position, DEFAULT_PANEL_THICKNESS)
	}
}

impl From<Vec3> for QuadCorner {
	fn from(position: Vec3) -> Self {
		Self::at(position)
	}
}

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
	pub a0: QuadCorner,
	pub a1: QuadCorner,
	pub b0: QuadCorner,
	pub b1: QuadCorner,
	pub joint_policy: QuadPanelJointPolicy,
}

impl QuadPanel {
	pub fn new(
		style: PanelStyle,
		a0: impl Into<QuadCorner>,
		a1: impl Into<QuadCorner>,
		b0: impl Into<QuadCorner>,
		b1: impl Into<QuadCorner>,
		joint_policy: QuadPanelJointPolicy,
	) -> Self {
		Self {
			style,
			a0: a0.into(),
			a1: a1.into(),
			b0: b0.into(),
			b1: b1.into(),
			joint_policy,
		}
	}

	pub fn rough_stone(
		a0: impl Into<QuadCorner>,
		a1: impl Into<QuadCorner>,
		b0: impl Into<QuadCorner>,
		b1: impl Into<QuadCorner>,
	) -> Self {
		Self::new(
			PanelStyle::RoughStonework,
			a0,
			a1,
			b0,
			b1,
			QuadPanelJointPolicy::default(),
		)
	}

	pub fn shepherds_thatch(
		a0: impl Into<QuadCorner>,
		a1: impl Into<QuadCorner>,
		b0: impl Into<QuadCorner>,
		b1: impl Into<QuadCorner>,
	) -> Self {
		Self::new(
			PanelStyle::ShepherdsThatch,
			a0,
			a1,
			b0,
			b1,
			QuadPanelJointPolicy::default(),
		)
	}

	pub fn with_joint_policy(mut self, joint_policy: QuadPanelJointPolicy) -> Self {
		self.joint_policy = joint_policy;
		self
	}

	/// Average thickness along line A (\((t_{a0}+t_{a1})/2\)).
	pub fn thickness_a(&self) -> f32 {
		0.5 * (self.a0.thickness + self.a1.thickness)
	}

	/// Average thickness along line B (\((t_{b0}+t_{b1})/2\)).
	pub fn thickness_b(&self) -> f32 {
		0.5 * (self.b0.thickness + self.b1.thickness)
	}

	/// Shared triangle / crease thickness: average of line A and line B.
	pub fn triangle_thickness(&self) -> f32 {
		0.5 * (self.thickness_a() + self.thickness_b())
	}

	/// Triangle on line A toward \(b_1\): \((a_0, a_1, b_1)\).
	pub fn triangle_a(&self) -> TessellatedTrianglePanel {
		TessellatedTrianglePanel::new(
			self.style,
			self.a0.position,
			self.a1.position,
			self.b1.position,
		)
	}

	/// Triangle on line B toward \(a_0\): \((a_0, b_1, b_0)\).
	pub fn triangle_b(&self) -> TessellatedTrianglePanel {
		TessellatedTrianglePanel::new(
			self.style,
			self.a0.position,
			self.b1.position,
			self.b0.position,
		)
	}

	/// Unit normals of the two triangles, or [`None`] if either is degenerate.
	pub fn triangle_normals(&self) -> Option<(Vec3, Vec3)> {
		let n0 = triangle_normal(self.a0.position, self.a1.position, self.b1.position)?;
		let n1 = triangle_normal(self.a0.position, self.b1.position, self.b0.position)?;
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
	///
	/// Kit \(+Y\) runs \(a_0 \to b_1\); \(X/Z\) diameter equals [`Self::triangle_thickness`].
	pub fn joint_node(&self) -> Option<JointNode> {
		let kink = self.dihedral_kink()?;
		if kink < self.joint_policy.min_dihedral_rad {
			return None;
		}
		let (n0, n1) = self.triangle_normals()?;
		// Bisector in the crease plane gives a stable radial (+X) hint.
		let radial_hint = n0 + n1;
		let placement = JointPost::placed_along_crease(
			self.a0.position,
			self.b1.position,
			self.triangle_thickness(),
			radial_hint,
		)?;
		Some(JointNode::rough_stone_post(placement))
	}
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
	use bevy_math::{EulerRot, Quat};
	use richmond_building_components::BuildingComponents;
	use richmond_building_components::joints::JOINT_KIT_XZ;

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
	fn folded_joint_aligns_y_with_diagonal_and_xz_with_thickness() {
		// XZ triangle + YZ triangle share a0–b1; normals ⊥ → 90° dihedral.
		let thick = 0.25;
		let a0 = QuadCorner::new(Vec3::ZERO, thick);
		let a1 = QuadCorner::new(Vec3::new(1.0, 0.0, 0.0), thick);
		let b0 = QuadCorner::new(Vec3::new(0.0, 1.0, 0.0), thick);
		let b1 = QuadCorner::new(Vec3::new(0.0, 0.0, 1.0), thick);
		let quad = QuadPanel::rough_stone(a0, a1, b0, b1)
			.with_joint_policy(QuadPanelJointPolicy::default());
		let kink = quad.dihedral_kink().expect("kink");
		assert!(
			(kink - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
			"expected ~90° fold, got {kink}"
		);
		let joint = quad.joint_node().expect("joint");
		let p = &joint.placement;
		assert!((p.translation - a0.position).length() < 1e-4);
		let diag = (b1.position - a0.position).normalize();
		let rot = Quat::from_euler(EulerRot::YXZ, p.yaw, p.pitch, p.roll);
		let y_axis = rot * Vec3::Y;
		assert!(
			(y_axis - diag).length() < 1e-3 || (y_axis + diag).length() < 1e-3,
			"kit +Y should align with diagonal, got {y_axis:?} vs {diag:?}"
		);
		assert!((p.scale.y - (b1.position - a0.position).length()).abs() < 1e-4);
		let want_xz = thick / JOINT_KIT_XZ;
		assert!((p.scale.x - want_xz).abs() < 1e-4);
		assert!((p.scale.z - want_xz).abs() < 1e-4);
	}

	#[test]
	fn line_thickness_averages_corners() {
		let quad = QuadPanel::rough_stone(
			QuadCorner::new(Vec3::ZERO, 0.2),
			QuadCorner::new(Vec3::new(1.0, 0.0, 0.0), 0.4),
			QuadCorner::new(Vec3::new(0.0, 1.0, 0.0), 0.6),
			QuadCorner::new(Vec3::new(0.0, 0.0, 1.0), 0.8),
		);
		assert!((quad.thickness_a() - 0.3).abs() < 1e-5);
		assert!((quad.thickness_b() - 0.7).abs() < 1e-5);
		assert!((quad.triangle_thickness() - 0.5).abs() < 1e-5);
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
