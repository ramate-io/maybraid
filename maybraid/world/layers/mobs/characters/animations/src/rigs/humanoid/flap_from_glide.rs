use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{smoothstep, FlapFromGlide};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for FlapFromGlide {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let elapsed = elapsed.max(0.0);
		let blend = self.blend_secs.max(0.0);
		if blend <= f32::EPSILON || elapsed >= blend {
			self.flap.apply_for(rig, elapsed);
		} else {
			let weight = smoothstep(elapsed / blend);
			blend_clips(rig, &self.soar, self.soar_glide_progress(), &self.flap, elapsed, weight);
		}
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _elapsed: f32) -> Effects {
		ArmatureOffset::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::{Flapping, Soaring, Transition};

	fn tip(rig: &HumanoidV0Rig, name: &str) -> bevy::prelude::Vec3 {
		rig.character_point(name) + rig.character_length(name)
	}

	fn sample(entry: &FlapFromGlide, elapsed: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut rig, elapsed);
		rig
	}

	fn max_rotation_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).normalize().angle_between(b.rotation(name).normalize()))
			.fold(0.0, f32::max)
	}

	#[test]
	fn entry_and_exit_match_their_source_clips() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let at_start = sample(&entry, 0.0);
		let mut glide = HumanoidV0Rig::for_clip_test();
		Soaring::default().apply(&mut glide, entry.soar_glide_progress());
		assert!((tip(&at_start, "forearm.L") - tip(&glide, "forearm.L")).length() < 1e-4);
		let at_end = sample(&entry, entry.blend_secs);
		let mut flap = HumanoidV0Rig::for_clip_test();
		Flapping::default().apply(&mut flap, entry.blend_secs);
		assert!(max_rotation_delta(&at_end, &flap) < 1e-5);
		assert!(entry.effects_for(&at_end, 0.2).is_identity());
		Ok(())
	}

	#[test]
	fn middle_of_blend_moves_toward_flap_and_preserves_unwritten_bones() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default().with_blend_secs(0.3);
		let mid = sample(&entry, entry.blend_secs * 0.5);
		let start = sample(&entry, 0.0);
		let end = sample(&entry, entry.blend_secs);
		assert!((tip(&mid, "forearm.L") - tip(&start, "forearm.L")).length() > 0.01);
		assert!((tip(&mid, "forearm.L") - tip(&end, "forearm.L")).length() > 0.01);
		for name in ["root", "pelvis.L", "pelvis.R", "femur.L", "femur.R", "shin.L", "shin.R"] {
			let Some(id) = mid.binding.definition.id(name) else {
				anyhow::bail!("missing rig bone {name}");
			};
			assert!(
				mid.pose.rotation(id).normalize().dot(start.pose.rotation(id).normalize()).abs()
					> 1.0 - 1e-6,
				"unwritten {name} stays at rest"
			);
		}
		Ok(())
	}

	#[test]
	fn branch_boundary_is_continuous_and_sampling_is_repeatable() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let before = sample(&entry, entry.blend_secs - 1e-4);
		let at = sample(&entry, entry.blend_secs);
		let after = sample(&entry, entry.blend_secs + 1e-4);
		assert!((tip(&at, "forearm.L") - tip(&before, "forearm.L")).length() < 0.002);
		assert!((tip(&after, "forearm.L") - tip(&at, "forearm.L")).length() < 0.002);
		let mut rig = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut rig, 0.17);
		let captured = rig.pose.clone();
		entry.apply(&mut rig, 1.2);
		entry.apply(&mut rig, 0.17);
		assert_eq!(rig.pose, captured);
		let transition = Transition::from_visible(entry, captured, ArmatureOffset::IDENTITY);
		let mut interrupted = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut interrupted, 0.17, 0.0);
		assert_eq!(interrupted.pose, rig.pose);
		assert_eq!(interrupted.scratch.depth, 0);
		Ok(())
	}

	#[test]
	fn samples_are_smooth_at_60_and_240_hz() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let mut previous = sample(&entry, 0.0);
		let mut max_60 = 0.0_f32;
		for frame in 1..=60 {
			let current = sample(&entry, frame as f32 / 60.0);
			max_60 = max_60.max(max_rotation_delta(&current, &previous));
			assert_eq!(current.scratch.depth, 0);
			previous = current;
		}
		let mut previous = sample(&entry, 0.0);
		let mut max_240 = 0.0_f32;
		for frame in 1..=240 {
			let current = sample(&entry, frame as f32 / 240.0);
			max_240 = max_240.max(max_rotation_delta(&current, &previous));
			previous = current;
		}
		assert!(max_60.is_finite() && max_60 > 0.02);
		assert!(max_240 < max_60 * 0.35, "240 Hz {max_240} vs 60 Hz {max_60}");
		Ok(())
	}

	#[test]
	fn zero_blend_selects_flapping_without_division() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default().with_blend_secs(0.0);
		let actual = sample(&entry, 0.0);
		let mut expected = HumanoidV0Rig::for_clip_test();
		Flapping::default().apply(&mut expected, 0.0);
		assert!(max_rotation_delta(&actual, &expected) < 1e-5);
		Ok(())
	}
}
