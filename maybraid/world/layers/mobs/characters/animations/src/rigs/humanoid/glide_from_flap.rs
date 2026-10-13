use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{smoothstep, GlideFromFlap};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for GlideFromFlap {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let soar_progress = self.soar_glide_progress() + elapsed;
		let blend = self.blend_secs.max(f32::EPSILON);
		if elapsed < blend {
			let weight = smoothstep((elapsed / blend).clamp(0.0, 1.0));
			blend_clips(rig, &self.flap, elapsed, &self.soar, soar_progress, weight);
		} else {
			self.soar.apply_for(rig, soar_progress);
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
	use crate::animations::Transition;

	fn wing_tip(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("forearm.L") + rig.character_length("forearm.L")
	}

	#[test]
	fn start_matches_flapping_pose() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let mut flap = HumanoidV0Rig::for_clip_test();
		let mut start = HumanoidV0Rig::for_clip_test();
		Flapping::default().apply(&mut flap, 0.0);
		exit.apply(&mut start, 0.0);
		assert!(
			(wing_tip(&flap) - wing_tip(&start)).length() < 1e-3,
			"weight 0 should keep the flapping pose"
		);
		Ok(())
	}

	#[test]
	fn end_of_blend_matches_soaring_glide() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let t = exit.blend_secs;
		let mut blended = HumanoidV0Rig::for_clip_test();
		let mut glide = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut blended, t);
		Soaring::default().apply(&mut glide, exit.soar_glide_progress() + t);
		assert!(
			blended.rotation("shoulder.L").dot(glide.rotation("shoulder.L")).abs() > 1.0 - 1e-5,
			"blend end should match pure soaring glide sample"
		);
		Ok(())
	}

	#[test]
	fn mid_blend_is_between_flap_and_glide() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let mid_t = 0.1;
		assert!(mid_t < exit.blend_secs);
		let mut mid = HumanoidV0Rig::for_clip_test();
		let mut flap_only = HumanoidV0Rig::for_clip_test();
		let mut glide = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut mid, mid_t);
		Flapping::default().apply(&mut flap_only, mid_t);
		Soaring::default().apply(&mut glide, exit.soar_glide_progress() + mid_t);
		let mid_tip = wing_tip(&mid);
		let flap_tip = wing_tip(&flap_only);
		let glide_tip = wing_tip(&glide);
		let span = (flap_tip - glide_tip).length();
		assert!(span > 0.05, "pick a phase with visible separation, span={span}");
		let flap_dist = (mid_tip - flap_tip).length();
		let glide_dist = (mid_tip - glide_tip).length();
		assert!(
			flap_dist > 0.02 && glide_dist > 0.02,
			"mid blend should leave both endpoints, flap={flap_dist} glide={glide_dist}"
		);
		assert!(
			flap_dist < span && glide_dist < span,
			"mid sample should sit between flap and glide extremes"
		);
		Ok(())
	}

	#[test]
	fn after_blend_window_tracks_soaring_cycle() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let t = exit.blend_secs + 0.35;
		let mut composed = HumanoidV0Rig::for_clip_test();
		let mut soar = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut composed, t);
		Soaring::default().apply(&mut soar, exit.soar_glide_progress() + t);
		assert!(
			composed.rotation("shoulder.L").dot(soar.rotation("shoulder.L")).abs() > 1.0 - 1e-5,
			"past the blend band the clip should match soaring"
		);
		Ok(())
	}

	#[test]
	fn interrupted_transition_preserves_visible_pose() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let mut rig = HumanoidV0Rig::for_clip_test();
		Flapping::default().apply(&mut rig, 0.2);
		let from_pose = rig.pose.clone();
		let from_offset = ArmatureOffset::IDENTITY;
		let transition = Transition::from_visible(exit, from_pose, from_offset);
		transition.apply(&mut rig, exit.blend_secs * 0.35, 0.4);
		assert!(rig.posed_angle("shoulder.L") > 0.1);
		Ok(())
	}

	#[test]
	fn no_armature_offset() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let rig = HumanoidV0Rig::for_clip_test();
		let effects = exit.effects_for(&rig, 0.25);
		assert!(effects.is_identity());
		Ok(())
	}

	#[test]
	fn resampling_does_not_accumulate() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut once, 0.25);
		exit.apply(&mut twice, 0.25);
		exit.apply(&mut twice, 0.25);
		for name in once.animation_bone_names() {
			let a = once.rotation(name);
			let b = twice.rotation(name);
			assert!(a.dot(b).abs() > 1.0 - 1e-5, "re-sample must not drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn wing_rig_fixture_band_edge() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default();
		let soar = Soaring::default();
		let flap = Flapping::default();
		let mut at_band = HumanoidV0Rig::for_clip_test();
		let mut past_band = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut at_band, 0.4);
		exit.apply(&mut past_band, 0.41);
		let band_tip = wing_tip(&at_band);
		let past_tip = wing_tip(&past_band);
		let soar_at_40 = {
			let mut r = HumanoidV0Rig::for_clip_test();
			soar.apply(&mut r, exit.soar_glide_progress() + 0.4);
			wing_tip(&r)
		};
		let soar_at_41 = {
			let mut r = HumanoidV0Rig::for_clip_test();
			soar.apply(&mut r, exit.soar_glide_progress() + 0.41);
			wing_tip(&r)
		};
		let band_delta = (band_tip - soar_at_40).length();
		let past_delta = (past_tip - soar_at_41).length();
		let mut just_before = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut just_before, 0.4 - 1e-4);
		let branch_edge = (wing_tip(&just_before) - band_tip).length();
		assert!(band_delta < 1e-3, "band end should match soaring@blend, delta={band_delta}");
		assert!(past_delta < 1e-3, "past band should match soaring@0.41, delta={past_delta}");
		assert!(
			branch_edge < 2e-3,
			"blend branch and pure-soar branch should meet at 0.4s, gap={branch_edge}"
		);

		let mut neutral = HumanoidV0Rig::for_clip_test();
		let mut quarter = HumanoidV0Rig::for_clip_test();
		flap.apply(&mut neutral, 0.0);
		flap.apply(&mut quarter, 0.1);
		let neutral_tip_z = wing_tip(&neutral).z;
		let quarter_tip_z = wing_tip(&quarter).z;
		assert!(
			(quarter_tip_z - neutral_tip_z).abs() > 0.05,
			"quarter-cycle stroke should move the wing tip"
		);
		Ok(())
	}
}
