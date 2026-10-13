use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{smoothstep, Flapping, GlideFromFlap, Soaring, GLIDE_FROM_FLAP_BLEND_SECS};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for GlideFromFlap {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let soar_progress = self.soar_glide_progress() + elapsed;
		let blend = self.blend_secs.max(f32::EPSILON);
		if elapsed < blend {
			let weight = smoothstep((elapsed / blend).clamp(0.0, 1.0));
			let flap_progress = self.flap_elapsed_start + elapsed;
			blend_clips(rig, &self.flap, flap_progress, &self.soar, soar_progress, weight);
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
		Flapping::default().apply(&mut flap_only, exit.flap_elapsed_start + mid_t);
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
	fn interrupted_from_visible_mid_band_preserves_pose() -> anyhow::Result<()> {
		let exit = GlideFromFlap::default().with_flap_elapsed_start(0.17);
		let mut rig = HumanoidV0Rig::for_clip_test();
		Flapping::default().apply(&mut rig, 0.17);
		let from_pose = rig.pose.clone();
		let from_tip = wing_tip(&rig);
		let transition = Transition::from_visible(exit, from_pose, ArmatureOffset::IDENTITY);
		let anim_t = exit.blend_secs * 0.35;

		let mut at_trans_zero = HumanoidV0Rig::for_clip_test();
		transition.apply(&mut at_trans_zero, anim_t, 0.0);
		assert!(
			(wing_tip(&at_trans_zero) - from_tip).length() < 1e-4,
			"transition weight 0 must restore the captured mid-flap pose"
		);

		let mut mid = HumanoidV0Rig::for_clip_test();
		let trans_t = 0.4;
		transition.apply(&mut mid, anim_t, trans_t);
		let mut clip_only = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut clip_only, anim_t);
		let clip_tip = wing_tip(&clip_only);
		let mid_tip = wing_tip(&mid);
		assert!(
			(mid_tip - from_tip).length() > 0.01 && (mid_tip - clip_tip).length() > 0.01,
			"mid-band interrupt should sit strictly between captured pose and clip sample"
		);
		assert!(
			(mid_tip - from_tip).length() < (clip_tip - from_tip).length(),
			"partial weight should not overshoot the clip endpoint"
		);
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

	fn wing_elbow(rig: &HumanoidV0Rig) -> (Vec3, Vec3) {
		(rig.character_point("forearm.L"), rig.character_point("forearm.R"))
	}

	fn wing_tips(rig: &HumanoidV0Rig) -> (Vec3, Vec3) {
		(
			rig.character_point("forearm.L") + rig.character_length("forearm.L"),
			rig.character_point("forearm.R") + rig.character_length("forearm.R"),
		)
	}

	fn assert_mirrored_wings(rig: &HumanoidV0Rig, label: &str) {
		let (el_l, el_r) = wing_elbow(rig);
		let (tip_l, tip_r) = wing_tips(rig);
		assert!(
			(el_l.x + el_r.x).abs() < 0.02 && (tip_l.x + tip_r.x).abs() < 0.02,
			"{label}: left/right wings should mirror about the sagittal plane"
		);
	}

	#[test]
	#[ignore = "prints wing-rig pose tables for transition review"]
	fn wing_rig_fixture_pose_report() {
		let soar = Soaring::default();
		let flap = Flapping::default();
		let glide_t = GlideFromFlap::default().soar_glide_progress();
		let times = [0.0, 0.1, 0.2, 0.4, 0.41];
		let entry_phases = [0.0, 0.1, 0.2, 0.3];
		eprintln!("soar_glide_progress={glide_t:.4}");
		for flap_start in entry_phases {
			let exit = GlideFromFlap::default().with_flap_elapsed_start(flap_start);
			eprintln!("\n=== entry flap phase {flap_start:.2} s ===");
			eprintln!("elapsed | GFF tip.L (x,y,z) | GFF elbow.L | Flap tip.L | Soar tip.L");
			for t in times {
				let mut gff = HumanoidV0Rig::for_clip_test();
				exit.apply(&mut gff, t);
				let mut flap_ref = HumanoidV0Rig::for_clip_test();
				let mut soar_ref = HumanoidV0Rig::for_clip_test();
				flap.apply(&mut flap_ref, flap_start + t);
				soar.apply(&mut soar_ref, glide_t + t);
				let (tip_l, tip_r) = wing_tips(&gff);
				let (el_l, el_r) = wing_elbow(&gff);
				let flap_l = wing_tips(&flap_ref).0;
				let soar_l = wing_tips(&soar_ref).0;
				eprintln!(
					"{t:4.2} | tip.L {tip_l:?} elbow.L {el_l:?} | flap tip.L {flap_l:?} | soar tip.L {soar_l:?}"
				);
				eprintln!("       | tip.R {tip_r:?} elbow.R {el_r:?}");
			}
		}
	}

	#[test]
	fn wing_rig_fixture_pose_continuity_and_band_edge() -> anyhow::Result<()> {
		let soar = Soaring::default();
		let flap = Flapping::default();
		let glide_t = GlideFromFlap::default().soar_glide_progress();
		let times = [0.0, 0.1, 0.2, 0.4, 0.41];
		let entry_phases = [0.0, 0.1, 0.2, 0.3];

		for flap_start in entry_phases {
			let exit = GlideFromFlap::default().with_flap_elapsed_start(flap_start);
			let mut flap_at_entry = HumanoidV0Rig::for_clip_test();
			flap.apply(&mut flap_at_entry, flap_start);
			let entry_tip = wing_tip(&flap_at_entry);
			let entry_elbow = wing_elbow(&flap_at_entry).0;

			let mut at_zero = HumanoidV0Rig::for_clip_test();
			exit.apply(&mut at_zero, 0.0);
			assert!(
				(wing_tip(&at_zero) - entry_tip).length() < 1e-3,
				"phase {flap_start}: elapsed=0 must match visible flapping, not phase 0"
			);
			assert!(
				(wing_elbow(&at_zero).0 - entry_elbow).length() < 1e-3,
				"phase {flap_start}: elbow at entry"
			);

			for t in times {
				let mut gff = HumanoidV0Rig::for_clip_test();
				exit.apply(&mut gff, t);
				assert_mirrored_wings(&gff, &format!("phase {flap_start} @ {t}s"));

				let mut flap_ref = HumanoidV0Rig::for_clip_test();
				let mut soar_ref = HumanoidV0Rig::for_clip_test();
				flap.apply(&mut flap_ref, flap_start + t);
				soar.apply(&mut soar_ref, glide_t + t);

				let (tip_l, tip_r) = wing_tips(&gff);
				let (flap_l, flap_r) = wing_tips(&flap_ref);
				let (soar_l, soar_r) = wing_tips(&soar_ref);
				if t > 0.0 && t < GLIDE_FROM_FLAP_BLEND_SECS {
					for (tip, flap_tip, soar_tip) in
						[(tip_l, flap_l, soar_l), (tip_r, flap_r, soar_r)]
					{
						let span = (flap_tip - soar_tip).length();
						if span < 0.02 {
							continue;
						}
						let flap_dist = (tip - flap_tip).length();
						let soar_dist = (tip - soar_tip).length();
						assert!(
							flap_dist > 1e-4 && soar_dist > 1e-4,
							"phase {flap_start} @ {t}s should sit between flap and soar samples"
						);
						assert!(
							flap_dist < span && soar_dist < span,
							"phase {flap_start} @ {t}s should stay inside the flap↔soar span"
						);
					}
				} else if t >= GLIDE_FROM_FLAP_BLEND_SECS {
					assert!(
						(tip_l - soar_l).length() < 1e-3 && (tip_r - soar_r).length() < 1e-3,
						"phase {flap_start} @ {t}s should match pure soaring"
					);
				}
			}
		}

		let exit = GlideFromFlap::default();
		let mut at_band = HumanoidV0Rig::for_clip_test();
		let mut past_band = HumanoidV0Rig::for_clip_test();
		exit.apply(&mut at_band, 0.4);
		exit.apply(&mut past_band, 0.41);
		let band_tip = wing_tip(&at_band);
		let past_tip = wing_tip(&past_band);
		let soar_at_40 = {
			let mut r = HumanoidV0Rig::for_clip_test();
			soar.apply(&mut r, glide_t + 0.4);
			wing_tip(&r)
		};
		let soar_at_41 = {
			let mut r = HumanoidV0Rig::for_clip_test();
			soar.apply(&mut r, glide_t + 0.41);
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
