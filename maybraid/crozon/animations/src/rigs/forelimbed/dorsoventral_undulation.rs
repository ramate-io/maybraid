use crozon_rigs::forelimbed::ForelimbedRig;
use crozon_rigs::Side;

use crate::animations::DorsoventralUndulation;
use crate::Animation;

const SEGMENT_COUNT: usize = 4;
const PITCH_SCALE: f32 = 0.4;
const FIN_FLEX: f32 = 0.1;

impl<R: ForelimbedRig> Animation<R> for DorsoventralUndulation {
	fn apply_for(&self, rig: &mut R, progress: f32) {
		let mut spine = rig.spine_pose();
		let pitches = [
			self.segment_pitch(progress, 0, SEGMENT_COUNT) * PITCH_SCALE,
			self.segment_pitch(progress, 1, SEGMENT_COUNT) * PITCH_SCALE,
			self.segment_pitch(progress, 2, SEGMENT_COUNT) * PITCH_SCALE,
			self.segment_pitch(progress, 3, SEGMENT_COUNT) * PITCH_SCALE,
		];

		// Pitch about Bevy X (twist on DEFAULT axes) for dorsoventral caudal beat.
		spine.upper_mid_spine =
			rig.articulate_on_rig_twisted(spine.upper_mid_spine, 0.0, 0.0, pitches[0] * 0.35);
		spine.upper_spine =
			rig.articulate_on_rig_twisted(spine.upper_spine, 0.0, 0.0, pitches[0] * 0.55);
		spine.lower_mid_spine =
			rig.articulate_on_rig_twisted(spine.lower_mid_spine, 0.0, 0.0, pitches[1]);
		spine.lower_spine = rig.articulate_on_rig_twisted(spine.lower_spine, 0.0, 0.0, pitches[2]);
		spine.tailbone = rig.articulate_on_rig_twisted(spine.tailbone, 0.0, 0.0, pitches[3]);
		rig.pose_spine(spine);

		let fin_phase = self.wave_phase(progress);
		for side in [Side::Left, Side::Right] {
			let paddle = (std::f32::consts::TAU * fin_phase).sin() * FIN_FLEX;
			let mut fin = rig.fin_pose(side);
			fin.shoulder = rig.articulate_on_rig(fin.shoulder, 0.0, paddle);
			fin.upper_arm = rig.articulate_on_rig(fin.upper_arm, 0.0, paddle * 0.6);
			rig.pose_fin(fin);
		}
	}
}
