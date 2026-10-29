//! Humanoid mapping for [`HandClap`](crate::animations::HandClap).

use bevy::math::Affine3A;
use bevy::prelude::{Quat, Vec3};
use character_rigs::articulation::{
	compose_parent_rotation, rotation_along_with_roll, TwoBoneAim, BONE_LENGTH_AXIS,
};
use character_rigs::authoring::{humanoid_bone_axis, ArmAim};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{HandClap, Idle};
use crate::Animation;

#[derive(Debug, Clone, Copy)]
struct ClapReach {
	along: Vec3,
	flex: f32,
	roll: f32,
}

impl ClapReach {
	fn solve(
		rig: &HumanoidV0Rig,
		clap: &HandClap,
		side: Side,
		progress: f32,
		lower_length: f32,
		reach_adjustment: Vec3,
	) -> Option<Self> {
		let (shoulder_name, humerus_name, forearm_name) = arm_bones(side);
		let definition = &rig.binding.definition;
		let rest = &rig.binding.effective_rest;
		let shoulder = definition.id(shoulder_name)?;
		let humerus = definition.id(humerus_name)?;
		let forearm = definition.id(forearm_name)?;
		let parent = definition.parent_rotation(rest, shoulder);
		let shoulder_local = rest.get(shoulder)?;
		let humerus_local = rest.get(humerus)?;
		let forearm_local = rest.get(forearm)?;
		let shoulder_delta = Quat::from_rotation_z(clap.shoulder_carry(side, progress));
		let mut shoulder_pose = shoulder_local;
		shoulder_pose.rotation = shoulder_delta * shoulder_pose.rotation;
		let shoulder_world = parent * shoulder_pose.rotation;
		let parent_affine = match definition.parent(shoulder) {
			Some(parent) => definition.affine_in_character(rest, parent)?,
			None => Affine3A::IDENTITY,
		};
		let shoulder_affine = parent_affine * shoulder_pose.compute_affine();
		let humerus_origin = shoulder_affine.transform_point3(humerus_local.translation);
		let humerus_rest_origin =
			definition.affine_in_character(rest, humerus)?.transform_point3(Vec3::ZERO);
		let forearm_rest_origin =
			definition.affine_in_character(rest, forearm)?.transform_point3(Vec3::ZERO);
		let upper_length = humerus_rest_origin.distance(forearm_rest_origin);
		let upper_back = definition.id("upper_back")?;
		let hand_target =
			definition.affine_in_character(rest, upper_back)?.transform_point3(Vec3::ZERO)
				+ clap.hand_target(side, progress);
		let reach = hand_target - humerus_origin + reach_adjustment;
		let pole = Vec3::X * side.sign();
		let aim = TwoBoneAim::reach(reach, pole, upper_length, lower_length)?;
		let rest_humerus = humerus_local.rotation;
		let aimed = rotation_along_with_roll(
			rest_humerus,
			shoulder_world.inverse() * aim.upper_along,
			0.0,
			BONE_LENGTH_AXIS,
		);
		let humerus_world = shoulder_world * aimed;
		let zero_roll_forearm = compose_parent_rotation(
			forearm_local.rotation,
			humanoid_bone_axis(forearm_name),
			0.0,
			aim.flex,
			0.0,
		);
		let from = zero_roll_forearm * BONE_LENGTH_AXIS;
		let to = humerus_world.inverse() * aim.lower_along;
		let roll = signed_angle_about_axis(from, to, BONE_LENGTH_AXIS)?;
		Some(Self { along: aim.upper_along, flex: aim.flex, roll })
	}
}

fn arm_bones(side: Side) -> (&'static str, &'static str, &'static str) {
	match side {
		Side::Left => ("shoulder.L", "humerus.L", "forearm.L"),
		Side::Right => ("shoulder.R", "humerus.R", "forearm.R"),
	}
}

fn signed_angle_about_axis(from: Vec3, to: Vec3, axis: Vec3) -> Option<f32> {
	let axis = axis.try_normalize()?;
	let from = (from - axis * from.dot(axis)).try_normalize()?;
	let to = (to - axis * to.dot(axis)).try_normalize()?;
	Some(axis.dot(from.cross(to)).atan2(from.dot(to)))
}

impl Animation<HumanoidV0Rig> for HandClap {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = Idle::default().sample_pose(0.0);
		rig.write_pose(&pose);
		let depth = rig.scratch.depth;
		rig.scratch.capture_from(depth, &rig.pose);
		let Some(upper_back) = rig.binding.definition.id("upper_back") else {
			return;
		};
		let amount = self.clap_amount(progress);
		if amount <= 0.0 {
			return;
		}
		let mut lower_lengths = rig.forearm_endpoints.map(|point| point.length());
		let mut reach_adjustments = [Vec3::ZERO; 2];
		for side in [Side::Left, Side::Right] {
			let (_, _, forearm_name) = arm_bones(side);
			if let Some(forearm) = rig.binding.definition.id(forearm_name) {
				if let Some(affine) =
					rig.binding.definition.affine_in_character(&rig.binding.effective_rest, forearm)
				{
					lower_lengths[side.index()] =
						affine.transform_vector3(rig.forearm_endpoints[side.index()]).length();
				}
			}
		}
		for side in [Side::Left, Side::Right] {
			pose.arm_mut(side).shoulder_lift = self.shoulder_carry(side, progress);
		}
		// Refine the isotropic two-bone seed against the actual affine FK. The
		// humerus thickness scale changes both distal length and hinge direction.
		for _ in 0..8 {
			for side in [Side::Left, Side::Right] {
				let arm = pose.arm_mut(side);
				if let Some(reach) = ClapReach::solve(
					rig,
					self,
					side,
					progress,
					lower_lengths[side.index()],
					reach_adjustments[side.index()],
				) {
					arm.elbow_flexion = reach.flex;
					arm.aim = Some(ArmAim { along: reach.along, roll: reach.roll });
				}
			}
			rig.write_pose(&pose);
			for side in [Side::Left, Side::Right] {
				let (_, _, forearm_name) = arm_bones(side);
				let Some(forearm) = rig.binding.definition.id(forearm_name) else {
					continue;
				};
				let Some(forearm_affine) =
					rig.binding.definition.affine_in_character(&rig.pose, forearm)
				else {
					continue;
				};
				let target = rig
					.binding
					.definition
					.affine_in_character(&rig.binding.effective_rest, upper_back)
					.map(|upper_back| {
						upper_back.transform_point3(Vec3::ZERO) + self.hand_target(side, progress)
					});
				if let Some(target) = target {
					let actual =
						forearm_affine.transform_point3(rig.forearm_endpoints[side.index()]);
					lower_lengths[side.index()] = forearm_affine
						.transform_vector3(rig.forearm_endpoints[side.index()])
						.length();
					reach_adjustments[side.index()] += target - actual;
				}
			}
		}
		rig.scratch.capture_to(depth, &rig.pose);
		if amount < 1.0 {
			// Blend complete orientations so approach/recovery follow a shortest rotation.
			rig.scratch.blend_saved(depth, amount, &mut rig.pose);
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;

	use super::*;
	use crate::animations::{HandClap, Transition};

	const CONTACT: f32 = 0.54;
	const STANDARD_FOREARM_ENDPOINTS: [Vec3; 2] =
		[Vec3::new(-0.00224, 0.37539, -0.00511), Vec3::new(-0.00091, 0.37579, -0.00535)];
	const FULL_FOREARM_ENDPOINTS: [Vec3; 2] =
		[Vec3::new(0.004056, 0.308892, 0.010478), Vec3::new(-0.022457, 0.312406, 0.013176)];

	fn joint(rig: &HumanoidV0Rig, name: &str) -> anyhow::Result<Vec3> {
		let Some(bone) = rig.binding.definition.id(name) else {
			anyhow::bail!("missing humanoid v0 joint {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, bone) else {
			anyhow::bail!("cannot compose humanoid v0 joint {name}");
		};
		Ok(affine.transform_point3(Vec3::ZERO))
	}

	/// Distal forearm point from the bound body's actual skinned terminal metadata.
	fn palm(rig: &HumanoidV0Rig, side: Side) -> anyhow::Result<Vec3> {
		let name = match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		};
		let Some(bone) = rig.binding.definition.id(name) else {
			anyhow::bail!("missing humanoid v0 joint {name}");
		};
		let Some(affine) = rig.binding.definition.affine_in_character(&rig.pose, bone) else {
			anyhow::bail!("cannot compose humanoid v0 forearm {name}");
		};
		Ok(affine.transform_point3(rig.forearm_endpoints[side.index()]))
	}

	fn posed(clap: &HandClap, progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		rig.forearm_endpoints = STANDARD_FOREARM_ENDPOINTS;
		clap.apply(&mut rig, progress);
		rig
	}

	fn assert_pose_near(
		a: &HumanoidV0Rig,
		b: &HumanoidV0Rig,
		tolerance: f32,
	) -> anyhow::Result<()> {
		for name in a.animation_bone_names() {
			let Some(id) = a.binding.definition.id(name) else {
				anyhow::bail!("missing humanoid v0 joint {name}");
			};
			let alignment =
				a.pose.rotation(id).normalize().dot(b.pose.rotation(id).normalize()).abs();
			assert!(alignment >= 1.0 - tolerance, "{name} quaternion alignment {alignment}");
		}
		Ok(())
	}

	#[test]
	fn contact_uses_composed_binds_and_keeps_palms_in_front_without_crossing() -> anyhow::Result<()>
	{
		let clap = HandClap;
		let rig = posed(&clap, CONTACT);
		let chest = joint(&rig, "upper_back")?;
		let head = joint(&rig, "upper_neck")?;
		let left = palm(&rig, Side::Left)?;
		let right = palm(&rig, Side::Right)?;
		assert!((left - right).length() < 0.04, "palms meet: {left:?} {right:?}");
		for (side, hand) in [(Side::Left, left), (Side::Right, right)] {
			let target = chest + clap.hand_target(side, CONTACT);
			assert!(
				(hand - target).length() < 0.025,
				"{side:?} palm reaches lower forward sternum target: {hand:?} {target:?}"
			);
			assert!(hand.y < chest.y - 0.045, "{side:?} palm below chest: {hand:?} {chest:?}");
			assert!(
				hand.z > chest.z + 0.27,
				"{side:?} palm in front of sternum: {hand:?} {chest:?}"
			);
			assert!(hand.y < head.y - 0.10, "{side:?} palm below upper neck: {hand:?} {head:?}");
			assert!(hand.x * side.sign() >= -0.01, "{side:?} palm stays on its side: {hand:?}");
		}
		Ok(())
	}

	#[test]
	fn pulse_samples_separate_palms_then_reach_contact_twice() -> anyhow::Result<()> {
		let clap = HandClap;
		let contact_a = posed(&clap, 0.30);
		let open_a = posed(&clap, 0.42);
		let contact_b = posed(&clap, 0.54);
		let open_b = posed(&clap, 0.66);
		let separation = |rig: &HumanoidV0Rig| -> anyhow::Result<f32> {
			Ok((palm(rig, Side::Left)? - palm(rig, Side::Right)?).length())
		};
		assert!(separation(&open_a)? > separation(&contact_a)? + 0.08);
		assert!(separation(&open_a)? > separation(&contact_b)? + 0.08);
		assert!(separation(&open_b)? > 0.08);
		for step in 0..=48 {
			let progress = 0.30 + step as f32 / 100.0;
			let rig = posed(&clap, progress);
			for side in [Side::Left, Side::Right] {
				assert!(
					palm(&rig, side)?.x * side.sign() >= -0.01,
					"no palm crosses at {progress}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn idle_endpoints_and_clamped_samples_are_exact() -> anyhow::Result<()> {
		let clap = HandClap;
		let mut neutral = HumanoidV0Rig::for_clip_test();
		neutral.forearm_endpoints = STANDARD_FOREARM_ENDPOINTS;
		Idle::default().apply(&mut neutral, 0.0);
		for progress in [-1.0, 0.0, 1.0, 2.0] {
			let mut rig = HumanoidV0Rig::for_clip_test();
			rig.forearm_endpoints = STANDARD_FOREARM_ENDPOINTS;
			let effects = clap.apply(&mut rig, progress);
			assert_eq!(rig.pose, neutral.pose, "progress {progress} returns full pose to Idle(0)");
			assert_eq!(effects, ArmatureOffset::IDENTITY);
			for side in [Side::Left, Side::Right] {
				let shoulder = joint(&rig, arm_bones(side).0)?;
				assert!(
					palm(&rig, side)?.y < shoulder.y - 0.20,
					"{side:?} hand hangs below the shoulder at {progress}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn motion_is_smooth_at_60hz_and_unwritten_bones_stay_at_rest() -> anyhow::Result<()> {
		let clap = HandClap;
		let mut neutral = HumanoidV0Rig::for_clip_test();
		neutral.forearm_endpoints = STANDARD_FOREARM_ENDPOINTS;
		Idle::default().apply(&mut neutral, 0.0);
		let mut previous = posed(&clap, 0.0);
		let mut max_palm_step = 0.0_f32;
		let mut largest_frame = 0;
		let mut max_elbow_step = 0.0_f32;
		for frame in 1..=60 {
			let rig = posed(&clap, frame as f32 / 60.0);
			for side in [Side::Left, Side::Right] {
				let palm_step = (palm(&rig, side)? - palm(&previous, side)?).length();
				if palm_step > max_palm_step {
					max_palm_step = palm_step;
					largest_frame = frame;
				}
				let forearm = match side {
					Side::Left => "forearm.L",
					Side::Right => "forearm.R",
				};
				max_elbow_step = max_elbow_step
					.max((rig.posed_angle(forearm) - previous.posed_angle(forearm)).abs());
			}
			previous = rig;
		}
		let rest_left = palm(&neutral, Side::Left)?;
		let clap_left = palm(&posed(&clap, CONTACT), Side::Left)?;
		assert!((rest_left - clap_left).length() > 0.25, "hands travel to chest contact");
		// The authored recovery lasts only 0.22 seconds. Refined sampling distinguishes
		// its peak speed from an orientation discontinuity.
		let mut refined_previous = posed(&clap, 0.0);
		let mut refined_max = 0.0_f32;
		let mut refined_elbow_max = 0.0_f32;
		for frame in 1..=240 {
			let refined = posed(&clap, frame as f32 / 240.0);
			for side in [Side::Left, Side::Right] {
				let (_, _, forearm) = arm_bones(side);
				refined_elbow_max = refined_elbow_max.max(
					(refined.posed_angle(forearm) - refined_previous.posed_angle(forearm)).abs(),
				);
				refined_max =
					refined_max.max(palm(&refined, side)?.distance(palm(&refined_previous, side)?));
			}
			refined_previous = refined;
		}
		assert!(
			refined_max < max_palm_step * 0.30,
			"fine samples must converge: {refined_max} vs {max_palm_step}"
		);
		assert!(
			max_palm_step.is_finite() && max_palm_step > 0.01,
			"60 Hz palm motion {max_palm_step} at frame {largest_frame}"
		);
		assert!(
			refined_elbow_max < max_elbow_step * 0.30,
			"fine elbow samples converge: {refined_elbow_max} vs {max_elbow_step}"
		);
		assert!(
			max_elbow_step.is_finite() && max_elbow_step > 0.01,
			"60 Hz elbow motion {max_elbow_step}"
		);
		for boundary in [0.30, 0.78] {
			let before = posed(&clap, boundary - 1e-4);
			let at = posed(&clap, boundary);
			let after = posed(&clap, boundary + 1e-4);
			for side in [Side::Left, Side::Right] {
				let p_before = palm(&before, side)?;
				let p_at = palm(&at, side)?;
				let p_after = palm(&after, side)?;
				assert!((p_at - p_before).length() < 0.002);
				assert!((p_after - p_at).length() < 0.002);
			}
		}
		let held = posed(&clap, CONTACT);
		assert_eq!(clap.effects_for(&held, CONTACT), ArmatureOffset::IDENTITY);
		for name in ["root", "pelvis.L", "pelvis.R", "femur.L", "femur.R", "shin.L", "shin.R"] {
			let Some(id) = held.binding.definition.id(name) else {
				anyhow::bail!("missing humanoid v0 joint {name}");
			};
			assert!(
				held.pose
					.rotation(id)
					.normalize()
					.dot(neutral.pose.rotation(id).normalize())
					.abs() > 1.0 - 1e-6,
				"unwritten {name} stays at fixed Idle phase zero"
			);
		}
		Ok(())
	}

	#[test]
	fn real_bind_positions_mirror_and_arms_remain_finite_at_changed_proportions(
	) -> anyhow::Result<()> {
		let clap = HandClap;
		let rig = posed(&clap, CONTACT);
		for (left_name, right_name) in [("shoulder.L", "shoulder.R"), ("forearm.L", "forearm.R")] {
			let left = joint(&rig, left_name)?;
			let right = joint(&rig, right_name)?;
			assert!((left.x + right.x).abs() < 0.03);
			assert!((left.y - right.y).abs() < 0.03);
			assert!((left.z - right.z).abs() < 0.03);
		}
		let mut long_arm = HumanoidV0Rig::for_clip_test();
		long_arm.forearm_endpoints = STANDARD_FOREARM_ENDPOINTS;
		long_arm.seed_rest("forearm.L", bevy::prelude::Transform::from_translation(Vec3::Y * 0.62));
		long_arm.seed_rest("forearm.R", bevy::prelude::Transform::from_translation(Vec3::Y * 0.62));
		clap.apply(&mut long_arm, CONTACT);
		assert!(
			(palm(&long_arm, Side::Left)? - palm(&long_arm, Side::Right)?).length() < 0.04,
			"contact adapts to longer effective-rest arm joints"
		);
		for side in [Side::Left, Side::Right] {
			let name = if side == Side::Left { "humerus.L" } else { "humerus.R" };
			assert!(long_arm.rotation(name).is_finite(), "finite aim for changed arm proportions");
		}
		Ok(())
	}

	#[test]
	fn braidman_scale_affine_reaches_chest_without_crossing() -> anyhow::Result<()> {
		let clap = HandClap;
		for endpoints in [STANDARD_FOREARM_ENDPOINTS, FULL_FOREARM_ENDPOINTS] {
			let mut rig = HumanoidV0Rig::for_clip_test();
			rig.forearm_endpoints = endpoints;
			let mut lumbar = bevy::prelude::Transform::from_translation(Vec3::Y * 0.25);
			lumbar.scale = Vec3::splat(0.8);
			rig.seed_rest("lumbar", lumbar);
			for name in ["humerus.L", "humerus.R"] {
				let mut humerus = bevy::prelude::Transform::from_translation(Vec3::Y * 0.35);
				humerus.scale = Vec3::new(0.8, 1.0, 0.8);
				rig.seed_rest(name, humerus);
			}
			clap.apply(&mut rig, CONTACT);
			let chest = joint(&rig, "upper_back")?;
			let head = joint(&rig, "upper_neck")?;
			let left = palm(&rig, Side::Left)?;
			let right = palm(&rig, Side::Right)?;
			assert!((left - right).length() < 0.01, "affine palms meet: {left:?} {right:?}");
			for (side, hand) in [(Side::Left, left), (Side::Right, right)] {
				let target = chest + clap.hand_target(side, CONTACT);
				assert!(
					(hand - target).length() < 0.025,
					"affine palm reaches lower forward sternum target: {side:?} {hand:?} {target:?}"
				);
				assert!(hand.y < chest.y - 0.045, "palm below chest landmark: {hand:?} {chest:?}");
				assert!(hand.y < head.y - 0.10, "palm below upper neck: {hand:?} {head:?}");
				assert!(
					hand.z > chest.z + 0.27,
					"palm stays in front of sternum: {hand:?} {chest:?}"
				);
				assert!(hand.x * side.sign() >= -0.005, "no side crossing: {side:?} {hand:?}");
			}
		}
		Ok(())
	}

	#[test]
	fn repeated_and_out_of_order_pose_samples_do_not_accumulate() -> anyhow::Result<()> {
		let clap = HandClap;
		let mut rig = HumanoidV0Rig::for_clip_test();
		clap.apply(&mut rig, 0.72);
		let first = rig.pose.clone();
		clap.apply(&mut rig, 0.12);
		clap.apply(&mut rig, 0.72);
		assert_eq!(rig.pose, first);
		Ok(())
	}

	#[test]
	fn visible_transition_preserves_capture_and_reaches_clap_pose() -> anyhow::Result<()> {
		let clap = HandClap;
		let mut source = HumanoidV0Rig::for_clip_test();
		let start_pose = source.pose.clone();
		let start_offset = ArmatureOffset::from_translation(Vec3::new(0.2, 0.0, 0.0));
		let transition = Transition::from_visible(clap, start_pose.clone(), start_offset);
		let at_zero = transition.apply(&mut source, CONTACT, 0.0);
		assert_eq!(source.pose, start_pose);
		assert_eq!(at_zero, start_offset);
		let mut target = HumanoidV0Rig::for_clip_test();
		transition.animation.apply(&mut target, CONTACT);
		let at_one = transition.apply(&mut source, CONTACT, 1.0);
		assert_pose_near(&source, &target, 1e-5)?;
		assert_eq!(at_one, ArmatureOffset::IDENTITY);
		let captured = source.pose.clone();
		let interruption =
			Transition::from_visible(HandClap, captured.clone(), ArmatureOffset::IDENTITY);
		interruption.apply(&mut source, 0.2, 0.0);
		assert_eq!(source.pose, captured, "interruption starts from the visible pose");
		Ok(())
	}

	#[test]
	fn nested_mix_keeps_its_saved_pose_when_hand_clap_refines_fk() -> anyhow::Result<()> {
		use crate::animations::Mix;

		let nested = Mix::new(Idle::default(), Mix::new(Idle::default(), HandClap, 0.8), 0.5);
		let flattened = Mix::new(Idle::default(), HandClap, 0.4);
		let mut actual = HumanoidV0Rig::for_clip_test();
		let mut expected = HumanoidV0Rig::for_clip_test();
		nested.apply(&mut actual, CONTACT);
		flattened.apply(&mut expected, CONTACT);
		assert_pose_near(&actual, &expected, 2e-4)?;
		assert_eq!(actual.scratch.depth, 0);
		Ok(())
	}
}
