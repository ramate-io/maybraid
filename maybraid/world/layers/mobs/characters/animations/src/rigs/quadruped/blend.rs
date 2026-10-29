use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::Animation;

/// Sample into reusable rig scratch and blend full local transforms.
pub(crate) fn blend_clips<A, B>(
	rig: &mut QuadrupedV0Rig,
	from: &A,
	from_progress: f32,
	to: &B,
	to_progress: f32,
	weight: f32,
) where
	A: Animation<QuadrupedV0Rig>,
	B: Animation<QuadrupedV0Rig>,
{
	let weight = weight.clamp(0.0, 1.0);
	if weight <= 0.0 {
		from.apply_for(rig, from_progress);
		return;
	}
	if weight >= 1.0 {
		to.apply_for(rig, to_progress);
		return;
	}
	let depth = rig.scratch.depth;
	rig.scratch.depth = depth + 1;
	from.apply_for(rig, from_progress);
	rig.scratch.capture_from(depth, &rig.pose);
	to.apply_for(rig, to_progress);
	rig.scratch.capture_to(depth, &rig.pose);
	rig.scratch.blend_saved(depth, weight, &mut rig.pose);
	rig.scratch.depth = depth;
}
