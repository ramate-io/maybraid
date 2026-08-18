use crozon_rigs::forelimbed::ForelimbedRig;
use crozon_rigs::Side;

use crate::animations::LateralUndulation;
use crate::Animation;

const SEGMENT_COUNT: usize = 4;
const YAW_SCALE: f32 = 0.45;
const FIN_SWING: f32 = 0.12;

impl<R: ForelimbedRig> Animation<R> for LateralUndulation {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let mut spine = rig.spine_pose();
		let yaws = [
			self.segment_yaw(progress, 0, SEGMENT_COUNT) * YAW_SCALE,
			self.segment_yaw(progress, 1, SEGMENT_COUNT) * YAW_SCALE,
			self.segment_yaw(progress, 2, SEGMENT_COUNT) * YAW_SCALE,
			self.segment_yaw(progress, 3, SEGMENT_COUNT) * YAW_SCALE,
		];

		spine.upper_mid_spine = rig.articulate_on_rig(spine.upper_mid_spine, yaws[0] * 0.35, 0.0);
		spine.upper_spine = rig.articulate_on_rig(spine.upper_spine, yaws[0] * 0.55, 0.0);
		spine.lower_mid_spine = rig.articulate_on_rig(spine.lower_mid_spine, yaws[1], 0.0);
		spine.lower_spine = rig.articulate_on_rig(spine.lower_spine, yaws[2], 0.0);
		spine.tailbone = rig.articulate_on_rig(spine.tailbone, yaws[3], 0.0);
		rig.pose_spine(spine);

		let fin_phase = self.wave_phase(progress);
		for side in [Side::Left, Side::Right] {
			let lateral = match side {
				Side::Left => 1.0,
				Side::Right => -1.0,
			};
			let paddle = (std::f32::consts::TAU * fin_phase).sin() * FIN_SWING * lateral;
			let mut fin = rig.fin_pose(side);
			fin.shoulder = rig.articulate_on_rig(fin.shoulder, paddle, 0.0);
			fin.upper_arm = rig.articulate_on_rig(fin.upper_arm, paddle * 0.6, 0.0);
			rig.pose_fin(fin);
		}
	}
}
