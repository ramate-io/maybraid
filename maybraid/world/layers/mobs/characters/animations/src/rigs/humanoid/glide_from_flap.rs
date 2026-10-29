use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{smoothstep, GlideFromFlap};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for GlideFromFlap {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let elapsed = elapsed.max(0.0);
		let soar_progress = self.soar_glide_progress() + elapsed;
		let blend = self.blend_secs.max(0.0);
		if blend <= f32::EPSILON || elapsed >= blend {
			self.soar.apply_for(rig, soar_progress);
		} else {
			let weight = smoothstep(elapsed / blend);
			blend_clips(
				rig,
				&self.flap,
				self.flap_elapsed_start + elapsed,
				&self.soar,
				soar_progress,
				weight,
			);
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

	fn sample(exit: &GlideFromFlap, elapsed: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut rig, elapsed);
		rig
	}

	fn max_rotation_delta(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).normalize().angle_between(b.rotation(name).normalize()))
			.fold(0.0, f32::max)
	}

	#[test]
	fn entry_preserves_flap_phase_and_exit_matches_quiet_glide() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default().with_flap_elapsed_start(0.17);
		let at_start = sample(&exit, 0.0);
		let mut flap = HumanoidV0Rig::for_clip_test();
		Flapping::default().apply(&mut flap, 0.17);
		assert!((tip(&at_start, "forearm.L") - tip(&flap, "forearm.L")).length() < 1e-4);
		let at_end = sample(&exit, exit.blend_secs);
		let mut glide = HumanoidV0Rig::for_clip_test();
		Soaring::default().apply(&mut glide, exit.soar_glide_progress() + exit.blend_secs);
		assert!(max_rotation_delta(&at_end, &glide) < 1e-5);
		assert!(exit.effects_for(&at_end, 0.2).is_identity());
		Ok(())
	}

	#[test]
	fn interrupted_blend_starts_at_captured_pose_and_resamples_without_drift() -> anyhow::Result<()>
	{
		let exit = GlideFromFlap::default().with_flap_elapsed_start(0.17);
		let mut source = HumanoidV0Rig::for_clip_test();
		Flapping::default().apply(&mut source, 0.17);
		let captured = source.pose.clone();
		let transition = Transition::from_visible(exit, captured.clone(), ArmatureOffset::IDENTITY);
		transition.apply(&mut source, 0.1, 0.0);
		assert_eq!(source.pose, captured);
		assert_eq!(source.scratch.depth, 0);
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut repeated = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut once, 0.23);
		exit.apply(&mut repeated, 0.23);
		exit.apply(&mut repeated, 0.9);
		exit.apply(&mut repeated, 0.23);
		assert_eq!(once.pose, repeated.pose);
		assert_eq!(repeated.scratch.depth, 0);
		Ok(())
	}

	#[test]
	fn blend_boundary_and_zero_window_are_well_defined() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let before = sample(&exit, exit.blend_secs - 1e-4);
		let at = sample(&exit, exit.blend_secs);
		let after = sample(&exit, exit.blend_secs + 1e-4);
		assert!((tip(&at, "forearm.L") - tip(&before, "forearm.L")).length() < 0.002);
		assert!((tip(&after, "forearm.L") - tip(&at, "forearm.L")).length() < 0.002);
		let zero = GlideFromFlap::default().with_blend_secs(0.0);
		let actual = sample(&zero, 0.0);
		let mut expected = HumanoidV0Rig::for_clip_test();
		Soaring::default().apply(&mut expected, zero.soar_glide_progress());
		assert!(max_rotation_delta(&actual, &expected) < 1e-5);
		Ok(())
	}

	#[test]
	fn samples_are_smooth_at_60_and_240_hz() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default().with_flap_elapsed_start(0.13);
		let mut previous = sample(&exit, 0.0);
		let mut max_60 = 0.0_f32;
		for frame in 1..=60 {
			let current = sample(&exit, frame as f32 / 60.0);
			max_60 = max_60.max(max_rotation_delta(&current, &previous));
			assert_eq!(current.scratch.depth, 0);
			previous = current;
		}
		let mut previous = sample(&exit, 0.0);
		let mut max_240 = 0.0_f32;
		for frame in 1..=240 {
			let current = sample(&exit, frame as f32 / 240.0);
			max_240 = max_240.max(max_rotation_delta(&current, &previous));
			previous = current;
		}
		assert!(max_60.is_finite() && max_60 > 0.02);
		assert!(max_240 < max_60 * 0.35, "240 Hz {max_240} vs 60 Hz {max_60}");
		Ok(())
	}
}
