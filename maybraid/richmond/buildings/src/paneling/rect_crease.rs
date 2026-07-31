//! Crease [`JointNode`]s between adjacent oriented rectangle bays.

use richmond_building_components::joints::{JointNode, JointPost};
use richmond_building_components::panels::dihedral_kink;

use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::rect_fit::OrientedRect;

/// Joint along the shared generator between two oriented bays, if the dihedral kink
/// meets `policy`.
///
/// Crease runs bottom→top along the shared station: average of `prev`’s trailing
/// edge \((b_0,b_1)\) and `next`’s leading edge \((a_0,a_1)\).
pub fn joint_along_bay_crease(
	prev: &OrientedRect,
	next: &OrientedRect,
	thickness: f32,
	policy: PanelComplexJointPolicy,
) -> Option<JointNode> {
	let kink = dihedral_kink(prev.normal, next.normal);
	if kink < policy.min_dihedral_rad {
		return None;
	}
	let start = (prev.b0 + next.a0) * 0.5;
	let end = (prev.b1 + next.a1) * 0.5;
	let radial_hint = prev.normal + next.normal;
	let placement = JointPost::placed_along_crease(start, end, thickness, radial_hint)?;
	Some(JointNode::rough_stone_post(placement))
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use crate::paneling::rect_fit::orient_rectangle;

	#[test]
	fn coplanar_bays_skip_default_policy() {
		let a = orient_rectangle(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), 2.0, 0.0).unwrap();
		let b = orient_rectangle(Vec3::new(0.0, 0.0, 2.0), Vec3::new(0.0, 0.0, 2.0), 2.0, 0.0)
			.unwrap();
		assert!(joint_along_bay_crease(&a, &b, 0.4, PanelComplexJointPolicy::default()).is_none());
	}

	#[test]
	fn folded_bays_emit_vertical_joint() {
		let a = orient_rectangle(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), 2.0, 0.0).unwrap();
		// Turn 90° in plan so normals (+X vs −Z) form a crease.
		let b = orient_rectangle(
			Vec3::new(0.0, 0.0, 2.0),
			Vec3::new(2.0, 0.0, 0.0),
			2.0,
			0.0,
		)
		.unwrap();
		let joint = joint_along_bay_crease(&a, &b, 0.4, PanelComplexJointPolicy::default())
			.expect("folded crease");
		// Kit +Y follows the crease; for roll-0 walls that is world +Y.
		let along = joint.placement.rotation() * Vec3::Y;
		assert!(
			along.y.abs() > 0.9,
			"joint should stand vertically, got along={along:?}"
		);
		assert!((joint.placement.translation - Vec3::new(0.0, 0.0, 2.0)).length() < 1e-3);
		assert!(joint_along_bay_crease(&a, &b, 0.4, PanelComplexJointPolicy::never()).is_none());
	}
}
