use character_rigs::authoring::PoseBuffer;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;

use crate::animations::Transition;
use crate::rigs::mix::mix_effects;
use crate::{Animation, Effects};

impl<A> Transition<A>
where
	A: Animation<QuadrupedV0Rig>,
{
	/// Latest-wins blend onto a quadruped rig. Same contract as the humanoid
	/// [`Transition::apply`](crate::rigs::transition): weight 0 keeps the
	/// captured pose and offset, weight 1 is the target clip.
	///
	/// Named separately because inherent `apply` is already bound to
	/// [`HumanoidV0Rig`](character_rigs::rigs::humanoid_v0::HumanoidV0Rig).
	pub fn apply_quadruped(
		&self,
		rig: &mut QuadrupedV0Rig,
		animation_progress: f32,
		transition_progress: f32,
	) -> Effects {
		let weight = self.weight(transition_progress);
		if weight <= 0.0 {
			rig.pose.copy_from(&self.from_pose);
			return self.from_offset;
		}
		self.animation.apply_for(rig, animation_progress);
		let effects = self.animation.effects_for(rig, animation_progress);
		if weight >= 1.0 {
			return mix_effects(self.from_offset, effects, 1.0);
		}
		rig.scratch.b.copy_from(&rig.pose);
		PoseBuffer::blend_into(&self.from_pose, &rig.scratch.b, weight, &mut rig.pose);
		mix_effects(self.from_offset, effects, weight)
	}
}
