use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{UprightWalk, Walk};
use crate::rigs::humanoid::gait_coordination::GaitCoordination;
use crate::rigs::humanoid::gait_knee::lerp_swing_knee;
use crate::rigs::humanoid::write_masks::{debug_assert_pose_within_mask, walk_write_mask};
use crate::Animation;

impl Walk {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		UprightWalk::from_walk(self).sample_pose(progress)
	}
}

impl Animation<HumanoidV0Rig> for Walk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, walk_write_mask(), "walk");
		rig.apply_masked_pose(&pose, walk_write_mask());
	}
}

impl UprightWalk {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		sample_walk(self, progress, &mut pose);
		pose
	}
}

impl Animation<HumanoidV0Rig> for UprightWalk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, walk_write_mask(), "upright_walk");
		rig.apply_masked_pose(&pose, walk_write_mask());
	}
}

fn sample_walk(walk: &UprightWalk, progress: f32, pose: &mut HumanoidPose) {
	let coord = GaitCoordination::from_progress(progress);
	let phase = coord.phase();
	let left_arm_swing = -GaitCoordination::arm_swing(phase);
	let right_arm_swing = GaitCoordination::arm_swing(phase + 0.5);

	// Same-sign pelvis yaw sends the left hip forward with the left femur.
	// add_turn uses the same oscillator so the shoulder girdle counters it.
	// Waist flex, not root, so forward lean stays a pure sagittal root twist.
	coord.apply_spine_counter(pose, walk.torso_lean, walk.torso_turn, walk.side_lean);
	apply_leg(pose, Side::Left, coord, -1.0, walk);
	apply_leg(pose, Side::Right, coord, 1.0, walk);
	// Both elbows share one flexion sign. The opposite arm_down values are a
	// hang bias, not a mirror of the joint frame.
	apply_walk_arm(pose, Side::Left, left_arm_swing, coord, -walk.arm_down, walk);
	apply_walk_arm(pose, Side::Right, right_arm_swing, coord, walk.arm_down, walk);
	coord.apply_head(pose, walk.torso_lean, walk.torso_turn, walk.side_lean, walk.head_stabilize);
}

fn apply_leg(
	pose: &mut HumanoidPose,
	side: Side,
	coord: GaitCoordination,
	lift_sign: f32,
	walk: &UprightWalk,
) {
	let cycle = coord.phase();
	let phase = if side == Side::Left { cycle } else { cycle + side.phase_offset() };
	let swing = GaitCoordination::thigh_swing(phase);
	// Coordinated yaw: both pelvis bones share one parent-space turn so the
	// hips get opposite character-Z (measured: same-sign turn is yaw).
	// Same-sign lateral is a roll/hike. Opposite-sign lateral lifts both hips
	// (measured: L+ R- raises the whole pelvis) and is the vertical bob.
	coord.apply_pelvis_yaw_and_bob(
		pose,
		side,
		swing,
		lift_sign,
		walk.hip_swing,
		walk.hip_lift,
		walk.body_bob,
	);
	let leg = pose.leg_mut(side);
	leg.hip_flexion = swing * walk.stride;
	leg.hip_abduction = -swing * walk.femur_medial_counter * lift_sign;
	leg.knee_flexion = knee_flex(phase, walk);
	// Trailing-leg pelvis pitch reads as heel-to-toe; swing > 0 is femur back.
	leg.pelvis_flexion = swing.max(0.0) * walk.foot_roll;
}

fn apply_walk_arm(
	pose: &mut HumanoidPose,
	side: Side,
	arm_swing_value: f32,
	coord: GaitCoordination,
	humerus_flex: f32,
	walk: &UprightWalk,
) {
	coord.apply_arm_across(
		pose,
		side,
		arm_swing_value,
		humerus_flex,
		walk.shoulder_swing,
		walk.shoulder_lift,
		walk.humerus_swing_scale,
		walk.arm_across,
		GaitCoordination::elbow_flex(
			arm_swing_value,
			coord.phase(),
			-1.0,
			walk.elbow_bend,
			walk.elbow_pump,
			walk.elbow_cycle,
		),
	);
}

/// Soft knee on stance; shared swing envelope peaks mid-stride for toe clearance.
fn knee_flex(leg_phase: f32, walk: &UprightWalk) -> f32 {
	lerp_swing_knee(leg_phase, walk.knee_stance_bend, walk.knee_swing_bend)
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use character_rigs::authoring::resolve_humanoid;
	use std::hint::black_box;
	use std::time::Instant;

	use super::*;
	use crate::animations::{Idle, Run};
	use crate::rigs::humanoid::write_masks::{idle_write_mask, run_write_mask, walk_write_mask};

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_walk = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			Walk::default().apply(&mut from_walk, phase);
			UprightWalk::default().apply(&mut from_upright, phase);

			for name in from_walk.animation_bone_names() {
				let walk_rot = from_walk.rotation(name);
				let upright_rot = from_upright.rotation(name);
				assert!(
					walk_rot.dot(upright_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn walk_delegates_to_upright_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn walk_animates_femur_in_the_sagittal_plane() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("femur.L") > 0.05, "stride leaves rest");
	}

	#[test]
	fn walk_legs_are_half_cycle_out_of_phase() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		UprightWalk::default().apply(&mut rig, 0.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
	}

	#[test]
	fn walk_stride_is_smaller_than_run() {
		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		Walk::default().apply(&mut walk_rig, 0.0);
		Run::default().apply(&mut run_rig, 0.0);

		assert!(
			walk_rig.posed_angle("femur.L") < run_rig.posed_angle("femur.L"),
			"walk stride should be smaller than run"
		);
	}

	#[test]
	fn walk_keeps_knee_closer_to_extended_than_run() {
		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		Walk::default().apply(&mut walk_rig, 0.75);
		Run::default().apply(&mut run_rig, 0.75);

		assert!(
			walk_rig.posed_angle("shin.L") < run_rig.posed_angle("shin.L"),
			"walk knee should stay closer to rest than run"
		);
	}

	#[test]
	fn walk_applies_forward_torso_lean() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		let root = tip(&rig, "root");
		assert!(root.z > 0.05, "forward lean goes to +Z, got {root:?}");
		assert!(root.x.abs() < 1e-3, "lean must not yaw, got {root:?}");
	}

	#[test]
	fn walk_femur_counters_hip_swing_out() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("pelvis.L") > 0.0, "pelvis yaws");
		assert!(rig.posed_angle("femur.L") > 0.0, "femur strides");
	}

	#[test]
	fn walk_stance_leg_has_soft_knee_bend() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("shin.L") > 0.0, "stance knee flexes");
	}

	#[test]
	fn walk_knee_flex_is_continuous_across_stride() {
		use crate::rigs::humanoid::gait_knee::SWING_KNEE_LIFT_SPAN;

		let walk = UprightWalk::default();
		let samples = 120;
		let max_step = (walk.knee_swing_bend - walk.knee_stance_bend) * std::f32::consts::PI
			/ (SWING_KNEE_LIFT_SPAN * samples as f32)
			+ 1e-4;
		let mut prev = knee_flex(0.0, &walk);
		for i in 1..=samples {
			let phase = i as f32 / samples as f32;
			let flex = knee_flex(phase, &walk);
			assert!((flex - prev).abs() < max_step, "knee snap at phase {phase}: {prev} -> {flex}");
			prev = flex;
		}
	}

	#[test]
	fn walk_knee_peak_precedes_late_swing_contact() {
		use crate::rigs::humanoid::gait_knee::{SWING_KNEE_LIFT_SPAN, SWING_KNEE_LIFT_START};

		let walk = UprightWalk::default();
		let expected_peak = SWING_KNEE_LIFT_START + SWING_KNEE_LIFT_SPAN * 0.5;
		let samples = 120;
		let mut peak_phase = 0.0;
		let mut peak_flex = knee_flex(0.0, &walk);
		for i in 1..=samples {
			let phase = i as f32 / samples as f32;
			let flex = knee_flex(phase, &walk);
			if flex > peak_flex {
				peak_flex = flex;
				peak_phase = phase;
			}
		}

		assert!(
			(peak_phase - expected_peak).abs() < 0.02,
			"expected peak near {expected_peak}, got {peak_phase}"
		);
		assert!(peak_phase < 0.75, "knee peak should precede late-swing contact");

		let mut rig = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut rig, peak_phase);
		assert!(
			rig.posed_angle("shin.L") > walk.knee_stance_bend + 0.4,
			"swing knee should flex at peak"
		);

		let mut late_neutral = None;
		for i in 0..=samples {
			let phase = i as f32 / samples as f32;
			if phase < 0.5 {
				continue;
			}
			let mut sample = HumanoidV0Rig::for_clip_test();
			Walk::default().apply(&mut sample, phase);
			if sample.character_length("femur.L").z.abs() < 0.02 {
				late_neutral = Some(phase);
				break;
			}
		}
		let contact = late_neutral.expect("femur should recross neutral late in stride");
		assert!(
			peak_phase < contact - 0.05,
			"peak knee {peak_phase} should clear before contact at {contact}"
		);
	}

	#[test]
	fn walk_knee_lift_envelope_matches_both_legs() {
		let walk = UprightWalk::default();
		for (global, shin) in [(0.60, "shin.L"), (0.10, "shin.R")] {
			let mut rig = HumanoidV0Rig::for_clip_test();
			Walk::default().apply(&mut rig, global);
			assert!(
				(rig.posed_angle(shin) - walk.knee_swing_bend).abs() < 0.05,
				"{shin} at global {global}"
			);
		}
	}

	#[test]
	fn walk_vertical_bob_comes_mostly_from_hips() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(
			rig.posed_angle("pelvis.L") > rig.posed_angle("shoulder.L"),
			"vertical bob comes mostly from hips"
		);
	}

	fn assert_masked_matches_full_resolve(
		pose: &character_rigs::authoring::HumanoidPose,
		mask: u32,
	) {
		let mut full = HumanoidV0Rig::imported();
		resolve_humanoid(pose, &full.binding, &mut full.pose);
		let mut masked = HumanoidV0Rig::imported();
		masked.apply_masked_pose(pose, mask);
		assert_eq!(full.pose.local, masked.pose.local, "masked resolve must match full");
	}

	fn walk_at(progress: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Walk::default().apply_for(&mut rig, progress);
		rig
	}

	fn mean_hip_y(rig: &HumanoidV0Rig) -> f32 {
		0.5 * (rig.character_point("femur.L").y + rig.character_point("femur.R").y)
	}

	#[test]
	fn walk_pelvis_yaws_left_hip_forward_at_phase_zero() -> anyhow::Result<()> {
		let rig = walk_at(0.0);
		let hip_l = rig.character_point("femur.L");
		let hip_r = rig.character_point("femur.R");
		assert!(hip_l.z - hip_r.z > 0.04, "left hip should lead in Z, L={hip_l:?} R={hip_r:?}");
		assert!(
			hip_l.z.abs() > hip_l.x.abs() - 0.24,
			"yaw is a Z split, not an X slide, L={hip_l:?}"
		);
		Ok(())
	}

	#[test]
	fn walk_shoulders_counter_pelvis_yaw() -> anyhow::Result<()> {
		let rig = walk_at(0.0);
		let hip_yaw = rig.character_point("femur.L").z - rig.character_point("femur.R").z;
		let shoulder = rig.rotation("upper_back") * Vec3::Z;
		assert!(hip_yaw > 0.04, "pelvis yaw L={hip_yaw}");
		assert!(
			hip_yaw * shoulder.x < 0.0,
			"shoulders must yaw opposite the pelvis, hip={hip_yaw} upperZ={shoulder:?}"
		);
		assert!(shoulder.x.abs() > 0.02, "torso twist must leave rest, {shoulder:?}");
		Ok(())
	}

	#[test]
	fn walk_body_bob_is_high_at_midstance() -> anyhow::Result<()> {
		let mid = mean_hip_y(&walk_at(0.0));
		let contact = mean_hip_y(&walk_at(0.25));
		let mid_again = mean_hip_y(&walk_at(0.5));
		assert!(
			mid > contact + 0.015,
			"midstance should sit above double-support, mid={mid} contact={contact}"
		);
		assert!(
			mid_again > contact + 0.015,
			"second midstance should sit above contact, mid={mid_again} contact={contact}"
		);
		Ok(())
	}

	#[test]
	fn walk_weight_shifts_toward_the_planted_side() -> anyhow::Result<()> {
		let left_plant = walk_at(0.0).rotation("lumbar") * Vec3::Y;
		let right_plant = walk_at(0.5).rotation("lumbar") * Vec3::Y;
		assert!(
			left_plant.x > 0.015,
			"phase 0 plants the left foot; waist should lean +X, {left_plant:?}"
		);
		assert!(
			right_plant.x < -0.015,
			"phase 0.5 plants the right foot; waist should lean −X, {right_plant:?}"
		);
		Ok(())
	}

	#[test]
	fn walk_head_is_more_level_than_the_chest() -> anyhow::Result<()> {
		let rig = walk_at(0.0);
		let chest = rig.rotation("root") * Vec3::Y;
		let head = rig.character_length("upper_neck");
		assert!(chest.z > 0.08, "walk still leans, {chest:?}");
		assert!(
			head.z.abs() < chest.z - 0.02,
			"neck should cancel lean, head={head:?} chest={chest:?}"
		);
		Ok(())
	}

	#[test]
	fn walk_forward_hand_comes_across_the_body() -> anyhow::Result<()> {
		let across = UprightWalk::default();
		let mut sagittal = UprightWalk::default();
		sagittal.arm_across = 0.0;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		across.apply_for(&mut a, 0.0);
		sagittal.apply_for(&mut b, 0.0);
		let across_right = hand_tip(&a, "forearm.R");
		let sagittal_right = hand_tip(&b, "forearm.R");
		let across_left = hand_tip(&a, "forearm.L");
		assert!(
			across_right.z > across_left.z + 0.3,
			"right hand is the forward one, L={across_left:?} R={across_right:?}"
		);
		assert!(
			across_right.x.abs() < sagittal_right.x.abs() - 0.02,
			"forward hand should come inward, across={across_right:?} sagittal={sagittal_right:?}"
		);
		Ok(())
	}

	#[test]
	fn walk_foot_roll_pulls_the_trailing_shin_back() -> anyhow::Result<()> {
		let rolled = UprightWalk::default();
		let mut flat = UprightWalk::default();
		flat.foot_roll = 0.0;
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		rolled.apply_for(&mut a, 0.5);
		flat.apply_for(&mut b, 0.5);
		let rolled_foot = shin_tail(&a, "shin.L");
		let flat_foot = shin_tail(&b, "shin.L");
		assert!(
			rolled_foot.z < flat_foot.z - 0.015,
			"trailing pelvis pitch should send the shin back, rolled={rolled_foot:?} flat={flat_foot:?}"
		);
		assert!(
			(rolled_foot.x - flat_foot.x).abs() < 0.04,
			"foot roll stays sagittal, rolled={rolled_foot:?} flat={flat_foot:?}"
		);
		Ok(())
	}

	#[test]
	fn walk_sample_does_not_accumulate() -> anyhow::Result<()> {
		let mut twice = HumanoidV0Rig::for_clip_test();
		Walk::default().apply_for(&mut twice, 0.37);
		Walk::default().apply_for(&mut twice, 0.37);
		let once = walk_at(0.37);
		for name in once.animation_bone_names() {
			assert!(
				twice.rotation(name).dot(once.rotation(name)).abs() > 1.0 - 1e-5,
				"second sample drifted on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn walk_is_continuous_at_mailbox_frame_intervals() -> anyhow::Result<()> {
		const DT: f32 = 1.0 / 60.0;
		const WALK_CYCLE_SPEED: f32 = 1.08;
		let step = DT * WALK_CYCLE_SPEED;
		let bones = ["root", "lumbar", "pelvis.L", "femur.L", "shin.L", "shoulder.L", "upper_neck"];
		let mut prev = walk_at(0.0);
		let mut progress = 0.0;
		for _ in 0..70 {
			progress += step;
			let now = walk_at(progress);
			for name in bones {
				let delta = now.rotation(name).angle_between(prev.rotation(name));
				assert!(delta < 0.12, "jerk on {name} at progress {progress}: delta={delta}");
			}
			prev = now;
		}
		Ok(())
	}

	#[test]
	fn walk_loop_seam_matches_the_cycle_wrap() -> anyhow::Result<()> {
		const DT: f32 = 1.0 / 60.0;
		const WALK_CYCLE_SPEED: f32 = 1.08;
		let step = DT * WALK_CYCLE_SPEED;
		let before = walk_at(1.0 - step);
		let at_one = walk_at(1.0);
		let at_zero = walk_at(0.0);
		let after = walk_at(step);
		for name in ["femur.L", "shin.L", "pelvis.L", "upper_back", "upper_neck"] {
			assert!(
				at_one.rotation(name).dot(at_zero.rotation(name)).abs() > 1.0 - 1e-5,
				"progress 1 and 0 must match on {name}"
			);
			let seam = at_zero.rotation(name).angle_between(before.rotation(name));
			let interior = after.rotation(name).angle_between(at_zero.rotation(name));
			assert!(
				(seam - interior).abs() < 0.04,
				"seam step on {name} should match a frame step, seam={seam} interior={interior}"
			);
		}
		Ok(())
	}

	#[test]
	fn cyclic_locomotion_masks_match_full_resolve() {
		let phases = [0.0, 0.11, 0.37, 0.62, 0.93];
		for &phase in &phases {
			assert_masked_matches_full_resolve(
				&Idle::default().sample_pose(phase),
				idle_write_mask(),
			);
			assert_masked_matches_full_resolve(
				&Walk::default().sample_pose(phase),
				walk_write_mask(),
			);
			assert_masked_matches_full_resolve(
				&Run::default().sample_pose(phase),
				run_write_mask(),
			);
		}
	}

	fn bench_locomotion_apply(use_masked: bool) -> Vec<u128> {
		const FRAMES: u32 = 2_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let clips: [(&str, f32); 3] = [("idle", 0.0), ("walk", 0.25), ("run", 0.5)];

		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter_mut().enumerate() {
					let progress =
						black_box((frame as f32 * 0.013 + (index as f32 * 0.07)).rem_euclid(1.0));
					let clip = clips[index as usize % clips.len()].0;
					match clip {
						"idle" => {
							let pose = Idle::default().sample_pose(progress);
							if use_masked {
								rig.apply_masked_pose(&pose, idle_write_mask());
							} else {
								rig.write_pose(&pose);
							}
						}
						"walk" => {
							let pose = Walk::default().sample_pose(progress);
							if use_masked {
								rig.apply_masked_pose(&pose, walk_write_mask());
							} else {
								rig.write_pose(&pose);
							}
						}
						"run" => {
							let pose = Run::default().sample_pose(progress);
							if use_masked {
								rig.apply_masked_pose(&pose, run_write_mask());
							} else {
								rig.write_pose(&pose);
							}
						}
						_ => {}
					}
					black_box(&rig.pose);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}

		run_ns.sort_unstable();
		run_ns
	}

	fn report_locomotion_bench(label: &str, run_ns: &[u128]) {
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		let mean = run_ns.iter().sum::<u128>() / run_ns.len() as u128;
		let spread = run_ns.last().expect("run") - min;
		eprintln!(
			"locomotion_resolve_microbench {label}: runs={runs:?} min={min} median={median} mean={mean} spread={spread} ns/sample",
			runs = run_ns,
		);
	}

	fn shin_tail(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name) + rig.character_length(name) * 0.5
	}

	fn hand_tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name) + rig.character_length(name) * 0.48
	}

	fn report_gait_pose(label: &str, apply: impl Fn(&mut HumanoidV0Rig, f32)) {
		eprintln!("=== {label} ===");
		for phase in [0.0, 0.125, 0.25, 0.375, 0.5, 0.625, 0.75, 0.875] {
			let mut rig = HumanoidV0Rig::for_clip_test();
			apply(&mut rig, phase);
			let hip_l = rig.character_point("femur.L");
			let hip_r = rig.character_point("femur.R");
			let foot_l = shin_tail(&rig, "shin.L");
			let foot_r = shin_tail(&rig, "shin.R");
			let hand_l = hand_tip(&rig, "forearm.L");
			let hand_r = hand_tip(&rig, "forearm.R");
			let femur_l = rig.character_length("femur.L");
			let femur_r = rig.character_length("femur.R");
			let root = rig.rotation("root") * Vec3::Y;
			let lumbar = rig.rotation("lumbar") * Vec3::Z;
			let upper = rig.rotation("upper_back") * Vec3::Z;
			let neck = rig.rotation("lower_neck") * Vec3::Y;
			let pelvis_l = rig.rotation("pelvis.L") * Vec3::Y;
			eprintln!(
				"phase={phase:.3} hipL={hip_l:?} hipR={hip_r:?} footL={foot_l:?} footR={foot_r:?} \
handL={hand_l:?} handR={hand_r:?} femurL={femur_l:?} femurR={femur_r:?} \
rootY={root:?} lumbarZ={lumbar:?} upperZ={upper:?} neckY={neck:?} pelvisLY={pelvis_l:?} \
angles femurL={:.3} shinL={:.3} shoulderL={:.3} humerusL={:.3} forearmL={:.3} pelvisL={:.3} root={:.3} lumbar={:.3} neck={:.3}",
				rig.posed_angle("femur.L"),
				rig.posed_angle("shin.L"),
				rig.posed_angle("shoulder.L"),
				rig.posed_angle("humerus.L"),
				rig.posed_angle("forearm.L"),
				rig.posed_angle("pelvis.L"),
				rig.posed_angle("root"),
				rig.posed_angle("lumbar"),
				rig.posed_angle("lower_neck"),
			);
		}
	}

	/// Isolated channels on `for_clip_test()` — field names are not the contract.
	#[test]
	fn posed_joint_channel_probe() -> anyhow::Result<()> {
		use character_rigs::authoring::HumanoidPose;

		let mut same_turn = HumanoidPose::default();
		same_turn.leg_mut(Side::Left).pelvis_turn = 0.2;
		same_turn.leg_mut(Side::Right).pelvis_turn = 0.2;
		let mut same = HumanoidV0Rig::for_clip_test();
		same.write_pose(&same_turn);
		let hip_l = same.character_point("femur.L");
		let hip_r = same.character_point("femur.R");
		assert!(
			hip_l.z < -0.03 && hip_r.z > 0.03,
			"same-sign pelvis_turn is yaw, L={hip_l:?} R={hip_r:?}"
		);

		let mut opp_turn = HumanoidPose::default();
		opp_turn.leg_mut(Side::Left).pelvis_turn = 0.2;
		opp_turn.leg_mut(Side::Right).pelvis_turn = -0.2;
		let mut opp = HumanoidV0Rig::for_clip_test();
		opp.write_pose(&opp_turn);
		let opp_l = opp.character_point("femur.L");
		let opp_r = opp.character_point("femur.R");
		assert!(
			(opp_l.z - opp_r.z).abs() < 0.01,
			"opposite pelvis_turn shares Z, L={opp_l:?} R={opp_r:?}"
		);

		let mut bob = HumanoidPose::default();
		bob.leg_mut(Side::Left).pelvis_lateral = 0.2;
		bob.leg_mut(Side::Right).pelvis_lateral = -0.2;
		let mut lifted = HumanoidV0Rig::for_clip_test();
		lifted.write_pose(&bob);
		assert!(
			lifted.character_point("femur.L").y > 0.03
				&& lifted.character_point("femur.R").y > 0.03,
			"opposite pelvis_lateral lifts both hips"
		);

		let mut lean = HumanoidPose::default();
		lean.apply_root(0.2);
		let mut pitched = HumanoidV0Rig::for_clip_test();
		pitched.write_pose(&lean);
		let root = pitched.rotation("root") * Vec3::Y;
		assert!(root.z > 0.15 && root.x.abs() < 1e-3, "root lean is sagittal, {root:?}");
		Ok(())
	}

	/// Baseline / after posed-joint dump. `cargo test -p character-animations posed_joint_gait_tables -- --nocapture`
	#[test]
	fn posed_joint_gait_tables() {
		report_gait_pose("WALK", |rig, p| {
			Walk::default().apply_for(rig, p);
		});
		report_gait_pose("RUN", |rig, p| {
			Run::default().apply_for(rig, p);
		});
	}

	/// `cargo test -p character-animations locomotion_resolve_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn locomotion_resolve_microbench() {
		eprintln!(
			"32 humanoid characters, idle/walk/run sample_pose + resolve, real Animation apply path"
		);
		report_locomotion_bench("legacy write_pose", &bench_locomotion_apply(false));
		report_locomotion_bench("masked partial resolve", &bench_locomotion_apply(true));
	}
}
