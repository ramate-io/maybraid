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
			blend_clips(rig, &self.soar, self.soar_glide_progress(), &self.flap, elapsed, weight);
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
		let entry =
			FlapFromGlide { blend_secs: FLAP_FROM_GLIDE_BLEND_SECS, ..FlapFromGlide::default() };
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

	fn wing_elbow(rig: &HumanoidV0Rig) -> (Vec3, Vec3) {
		(rig.character_point("forearm.L"), rig.character_point("forearm.R"))
	}

	fn wing_tips(rig: &HumanoidV0Rig) -> (Vec3, Vec3) {
		(
			rig.character_point("forearm.L") + rig.character_length("forearm.L"),
			rig.character_point("forearm.R") + rig.character_length("forearm.R"),
		)
	}

	#[test]
	fn wing_rig_fixture_pose_continuity_and_band_edge() -> anyhow::Result<()> {
		let entry = FlapFromGlide::default();
		let soar = Soaring::default();
		let flap = Flapping::default();
		let glide_t = entry.soar_glide_progress();
		let times = [0.0, 0.1, 0.2, 0.4, 0.41];

		let mut soar_rig = HumanoidV0Rig::for_clip_test();
		soar.apply(&mut soar_rig, glide_t);
		for t in times {
			let mut ffg = HumanoidV0Rig::for_clip_test();
			entry.apply(&mut ffg, t);
			let (el_l, el_r) = wing_elbow(&ffg);
			let (tip_l, tip_r) = wing_tips(&ffg);
			assert!(
				(el_l.x + el_r.x).abs() < 0.02 && (tip_l.x + tip_r.x).abs() < 0.02,
				"left/right wings should mirror about the sagittal plane"
			);
		}
		let mut at_zero = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut at_zero, 0.0);
		let zero_tip = wing_tip(&at_zero);
		let soar_tip = wing_tip(&soar_rig);
		assert!(
			(zero_tip - soar_tip).length() < 1e-3,
			"elapsed=0 must match soaring glide pose on wing-rig fixture"
		);

		let mut at_band = HumanoidV0Rig::for_clip_test();
		let mut past_band = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut at_band, 0.4);
		entry.apply(&mut past_band, 0.41);
		let band_tip = wing_tip(&at_band);
		let past_tip = wing_tip(&past_band);
		let flap_at_40 = {
			let mut r = HumanoidV0Rig::for_clip_test();
			flap.apply(&mut r, 0.4);
			wing_tip(&r)
		};
		let flap_at_41 = {
			let mut r = HumanoidV0Rig::for_clip_test();
			flap.apply(&mut r, 0.41);
			wing_tip(&r)
		};
		let band_delta = (band_tip - flap_at_40).length();
		let past_delta = (past_tip - flap_at_41).length();
		let mut just_before = HumanoidV0Rig::for_clip_test();
		entry.apply(&mut just_before, 0.4 - 1e-4);
		let branch_edge = (wing_tip(&just_before) - band_tip).length();
		assert!(band_delta < 1e-3, "band end should match flapping@0.4, delta={band_delta}");
		assert!(past_delta < 1e-3, "past band should match flapping@0.41, delta={past_delta}");
		assert!(
			branch_edge < 2e-3,
			"blend branch and pure-flap branch should meet at 0.4s, gap={branch_edge}"
		);

		// First stroke from glide: flap phase 0 matches neutral spread, then rises before downstroke.
		let mut neutral = HumanoidV0Rig::for_clip_test();
		let mut quarter = HumanoidV0Rig::for_clip_test();
		flap.apply(&mut neutral, 0.0);
		flap.apply(&mut quarter, 0.1);
		let neutral_tip_z = wing_tip(&neutral).z;
		let quarter_tip_z = wing_tip(&quarter).z;
		assert!(
			(quarter_tip_z - neutral_tip_z).abs() > 0.05,
			"quarter-cycle stroke should move the wing tip before the downstroke"
		);
		assert!(
			flap.flap_amount(0.0).abs() < 1e-5 && flap.flap_amount(0.1) > 0.2,
			"phase 0 is neutral then the first motion is the upstroke (positive flap_amount)"
		);
		Ok(())
	}
}
