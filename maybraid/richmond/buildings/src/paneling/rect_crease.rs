//! Crease [`JointNode`]s between adjacent oriented rectangle bays.

use richmond_building_components::joints::{JointNode, JointPost};
use richmond_building_components::panels::dihedral_kink;

use crate::paneling::panel_complex::PanelComplexJointPolicy;
use crate::paneling::rect_fit::OrientedRect;

/// Joint along the shared generator between two oriented bays, if the dihedral kink
/// meets `policy`.
///
/// Crease runs from the averaged \(a\)-rail ends to the averaged \(b\)-rail ends
/// (adjacent bays need not share exact vertices).
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
	let start = (prev.a1 + next.a0) * 0.5;
	let end = (prev.b1 + next.b0) * 0.5;
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
	fn folded_bays_emit_joint() {
		let a = orient_rectangle(Vec3::ZERO, Vec3::new(0.0, 0.0, 2.0), 2.0, 0.0).unwrap();
		// Turn 90° in plan so normals (+X vs −Z) form a crease.
		let b = orient_rectangle(
			Vec3::new(0.0, 0.0, 2.0),
			Vec3::new(2.0, 0.0, 0.0),
			2.0,
			0.0,
		)
		.unwrap();
		assert!(joint_along_bay_crease(&a, &b, 0.4, PanelComplexJointPolicy::default()).is_some());
		assert!(joint_along_bay_crease(&a, &b, 0.4, PanelComplexJointPolicy::never()).is_none());
	}
}
