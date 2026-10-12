use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{smoothstep, FlapFromGlide, Flapping, Soaring};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for FlapFromGlide {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let blend = self.blend_secs.max(f32::EPSILON);
		if elapsed < blend {
			let weight = smoothstep((elapsed / blend).clamp(0.0, 1.0));
			blend_clips(
				rig,
				&self.soar,
				self.soar_glide_progress(),
				&self.flap,
				elapsed,
				weight,
			);
		} else {
			self.flap.apply_for(rig, elapsed);
		}
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _elapsed: f32) -> Effects {
		Effects::IDENTITY
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::{Transition, FLAP_FROM_GLIDE_BLEND_SECS};

	fn wing_tip(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("forearm.L") + rig.character_length("forearm.L")
	}

	#[test]
	fn start_matches_soar_glide_pose() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let mut glide = HumanoidV0Rig::for_clip_test();
		let mut start = HumanoidV0Rig::for_clip_test();
		Soaring::default().apply(&mut glide, entry.soar_glide_progress());
		entry.apply(&mut start, 0.0);
		assert!(
			(wing_tip(&glide) - wing_tip(&start)).length() < 1e-3,
			"weight 0 should keep the soar glide pose"
		);
		Ok(())
	}

	#[test]
	fn end_of_blend_matches_flapping() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let t = entry.blend_secs;
		let mut blended = HumanoidV0Rig::for_clip_test();
		let mut flap = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut blended, t);
		Flapping::default().apply(&mut flap, t);
		assert!(
			blended.rotation("shoulder.L").dot(flap.rotation("shoulder.L")).abs() > 1.0 - 1e-5,
			"blend end should match pure flapping sample"
		);
		Ok(())
	}

	#[test]
	fn mid_blend_is_between_glide_and_flap() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		// Quarter flap cycle (peak stroke) while still inside the blend band.
		let mid_t = 0.1;
		assert!(mid_t < entry.blend_secs);
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut glide = HumanoidV0Rig::for_clip_test();
		let mut flap_only = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut mid, mid_t);
		Soaring::default().apply(&mut glide, entry.soar_glide_progress());
		Flapping::default().apply(&mut flap_only, mid_t);
		let mid_tip = wing_tip(&mid);
		let glide_tip = wing_tip(&glide);
		let flap_tip = wing_tip(&flap_only);
		let span = (glide_tip - flap_tip).length();
		assert!(span > 0.15, "pick flap phase with visible stroke, span={span}");
		let glide_dist = (mid_tip - glide_tip).length();
		let flap_dist = (mid_tip - flap_tip).length();
		assert!(
			glide_dist > 0.02 && flap_dist > 0.02,
			"mid blend should leave both endpoints, glide={glide_dist} flap={flap_dist}"
		);
		assert!(
			glide_dist < span && flap_dist < span,
			"mid sample should sit between glide and flap extremes"
		);
		Ok(())
	}

	#[test]
	fn after_blend_window_tracks_flapping_cycle() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let t = entry.blend_secs + 0.35;
		let mut composed = HumanoidV0Rig::for_clip_test();
		let mut flap = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut composed, t);
		Flapping::default().apply(&mut flap, t);
		assert!(
			composed.rotation("shoulder.L").dot(flap.rotation("shoulder.L")).abs() > 1.0 - 1e-5,
			"past the blend band the clip should match flapping"
		);
		Ok(())
	}

	#[test]
	fn blend_weight_endpoints() -> anyhow::Result<()> {
		let entry = FlapFromGlide {
			blend_secs: FLAP_FROM_GLIDE_BLEND_SECS,
			..FlapFromGlide::default()
		};
		let mut at_zero = HumanoidV0Rig::for_clip_test();
		let mut late = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut at_zero, 0.0);
		entry.apply(&mut late, entry.blend_secs * 0.85);
		let zero_tip = wing_tip(&at_zero);
		let late_tip = wing_tip(&late);
		assert!(
			(late_tip - zero_tip).length() > 0.08,
			"smoothstep weight should pull wings toward the flapping stroke before the band ends"
		);
		Ok(())
	}

	#[test]
	fn interrupted_transition_preserves_visible_pose() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let mut rig = HumanoidV0Rig::for_clip_test();
		Soaring::default().apply(&mut rig, entry.soar_glide_progress());
		let from_pose = rig.pose.clone();
		let from_offset = ArmatureOffset::IDENTITY;
		let transition = Transition::from_visible(entry, from_pose, from_offset);
		transition.apply(&mut rig, entry.blend_secs * 0.35, 0.4);
		assert!(rig.posed_angle("shoulder.L") > 0.1);
		Ok(())
	}

	#[test]
	fn no_armature_offset() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let rig = HumanoidV0Rig::for_clip_test();
		let effects = entry.effects_for(&rig, 0.25);
		assert!(effects.is_identity());
		Ok(())
	}
}
