//! Humanoid mapping for [`SpotScan`](crate::animations::SpotScan).

use bevy::prelude::Vec3;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Idle, SpotScan};
use crate::Animation;

impl Animation<HumanoidV0Rig> for SpotScan {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut target = Idle::default().sample_pose(0.0);
		target.spine.add_turn(self.torso_turn(progress));
		target.spine.add_root_forward(self.forward_lean(progress));
		target.apply_neck_twisted(
			self.neck_turn(progress) * 0.6,
			0.0,
			self.head_dip(progress),
			self.neck_turn(progress) * 0.4,
			0.0,
			0.0,
		);
		let knee = self.knee_flex(progress);
		for side in [Side::Left, Side::Right] {
			target.apply_leg(side, knee * 0.35, knee);
			target.leg_mut(side).pelvis_lateral += self.pelvis_shift(progress) * side.sign();
		}
		rig.write_pose(&target);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> ArmatureOffset {
		let drop = self.vertical_drop(progress, rig.segment_lengths);
		if drop > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, -drop, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::{Transform, Vec3};
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	fn idle() -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply_for(&mut rig, 0.0);
		rig
	}

	fn sample(progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effect = SpotScan.apply(&mut rig, progress);
		(rig, effect)
	}

	fn direction(rig: &HumanoidV0Rig, bone: &str, local: Vec3) -> anyhow::Result<Vec3> {
		let Some(id) = rig.binding.definition.id(bone) else {
			anyhow::bail!("missing {bone}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, id) else {
			anyhow::bail!("cannot compose {bone}");
		};
		Ok(affine.transform_vector3(local).normalize_or_zero())
	}

	fn max_rotation_step(a: &HumanoidV0Rig, b: &HumanoidV0Rig) -> f32 {
		a.animation_bone_names()
			.map(|name| a.rotation(name).angle_between(b.rotation(name)))
			.fold(0.0_f32, f32::max)
	}

	#[test]
	fn spot_scan_endpoints_and_clamps_match_idle_with_identity_effects() -> anyhow::Result<()> {
		let rest = idle();
		for progress in [0.0, 1.0, -1.0, 2.0] {
			let (rig, effect) = sample(progress);
			assert_eq!(rig.pose, rest.pose);
			assert_eq!(effect, ArmatureOffset::IDENTITY);
		}
		Ok(())
	}

	#[test]
	fn spot_scan_neck_leads_mirrored_torso_and_keeps_arms_idle() -> anyhow::Result<()> {
		for (progress, expected_sign) in
			[(SpotScan::left_peak(), 1.0), (SpotScan::right_peak(), -1.0)]
		{
			let (rig, effect) = sample(progress);
			let neck = direction(&rig, "upper_neck", Vec3::Z)?;
			let torso = direction(&rig, "upper_back", Vec3::Z)?;
			assert!(neck.x * expected_sign > 0.08, "neck scans {neck:?}");
			assert!(torso.x * expected_sign > 0.02, "torso follows {torso:?}");
			assert!(neck.x.abs() > torso.x.abs(), "neck leads torso {neck:?} {torso:?}");
			assert!(effect.0.translation.y <= 0.0, "scan does not lift the character");
			let rest = idle();
			for bone in ["humerus.L", "humerus.R", "forearm.L", "forearm.R"] {
				assert!(
					rig.rotation(bone).normalize().dot(rest.rotation(bone).normalize()).abs()
						> 1.0 - 1e-5,
					"{bone} remains Idle"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn spot_scan_has_soft_knees_and_a_bounded_foot_drop_effect() -> anyhow::Result<()> {
		let rest = idle();
		let (posed, effect) = sample(0.34);
		assert!(posed.posed_angle("shin.L") > rest.posed_angle("shin.L") + 0.02);
		assert!((posed.posed_angle("shin.L") - posed.posed_angle("shin.R")).abs() < 1e-4);
		assert!(
			effect.0.translation.y < 0.0 && effect.0.translation.y > -0.08,
			"bounded plant correction {effect:?}"
		);
		Ok(())
	}

	#[test]
	fn spot_scan_temporal_samples_refine_at_240_hz() -> anyhow::Result<()> {
		let rest = idle();
		let mut coarse_previous = rest.clone();
		let mut fine_previous = rest.clone();
		let mut coarse_max = 0.0_f32;
		let mut fine_max = 0.0_f32;
		for index in 1..=60 {
			let (rig, _) = sample(index as f32 / 60.0);
			coarse_max = coarse_max.max(max_rotation_step(&coarse_previous, &rig));
			coarse_previous = rig;
		}
		for index in 1..=240 {
			let (rig, _) = sample(index as f32 / 240.0);
			fine_max = fine_max.max(max_rotation_step(&fine_previous, &rig));
			fine_previous = rig;
		}
		assert!(fine_max < coarse_max, "240 Hz {fine_max} should refine 60 Hz {coarse_max}");
		assert!(coarse_max < 0.2, "60 Hz step {coarse_max}");
		Ok(())
	}

	#[test]
	fn spot_scan_drop_remains_finite_with_longer_shins() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		for name in ["shin.L", "shin.R"] {
			let Some(id) = rig.binding.definition.id(name) else {
				anyhow::bail!("missing {name}");
			};
			let Some(rest) = rig.binding.effective_rest.local.get(id.index()).copied() else {
				anyhow::bail!("missing {name} rest");
			};
			rig.seed_rest(name, Transform { scale: rest.scale * 1.2, ..rest });
		}
		let effect = SpotScan.apply(&mut rig, 0.34);
		assert!(effect.0.translation.is_finite());
		assert!(rig.rotation("shin.L").is_finite() && rig.rotation("shin.R").is_finite());
		Ok(())
	}
}
