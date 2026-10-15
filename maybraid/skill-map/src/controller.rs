//! Stick-flick detector. Either stick while **RB** is held; a hold is not a flick.
//! Emits [`SkillMapFlick`].

use bevy::prelude::*;
use maybraid_input::{FlickLane, PadButton, VirtualPad};

use crate::SkillMapEnabled;

/// One completed flick. Direction is the stick throw; length is how hard (0..1+).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct SkillMapFlick(pub Vec2);

/// True while the skill-map steer modifier is down (Xbox **RB** / keyboard **V**).
///
/// The LB+RB chord is the Tanzer / mode-toggle hold, not a steer.
pub fn skill_map_steer_held(pad: &VirtualPad) -> bool {
	pad.pressed(PadButton::BumperFire) && !pad.bumper_chord_held()
}

/// Watches move and look sticks. One flick per sample, stronger stick wins if both land.
#[derive(Resource, Clone, Debug, Default)]
pub struct SkillMapController {
	move_lane: FlickLane,
	look_lane: FlickLane,
}

impl SkillMapController {
	pub fn reset(&mut self) {
		self.move_lane.reset();
		self.look_lane.reset();
	}

	pub fn sample(&mut self, t: f32, move_stick: Vec2, look_stick: Vec2) -> Option<Vec2> {
		let moved = self.move_lane.sample(t, move_stick);
		let looked = self.look_lane.sample(t, look_stick);
		match (moved, looked) {
			(Some(a), Some(b)) if b.length_squared() > a.length_squared() => Some(b),
			(Some(a), _) => Some(a),
			(None, Some(b)) => Some(b),
			(None, None) => None,
		}
	}
}

pub fn detect_skill_map_flicks(
	time: Res<Time>,
	pad: Option<Res<VirtualPad>>,
	enabled: Res<SkillMapEnabled>,
	mut controller: ResMut<SkillMapController>,
	mut flicks: MessageWriter<SkillMapFlick>,
) {
	if !enabled.0 {
		controller.reset();
		return;
	}
	let Some(pad) = pad else {
		return;
	};
	if !skill_map_steer_held(&pad) {
		controller.reset();
		return;
	}
	if let Some(flick) = controller.sample(time.elapsed_secs(), pad.move_stick, pad.look_analog) {
		flicks.write(SkillMapFlick(flick));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn throw(
		controller: &mut SkillMapController,
		t0: f32,
		stick: Vec2,
		on_move: bool,
	) -> Option<Vec2> {
		let mut last = None;
		for (i, mag) in [0.0, 0.75, 1.0, 0.4, 0.0].into_iter().enumerate() {
			let t = t0 + i as f32 * 0.04;
			let v = stick * mag;
			last = if on_move {
				controller.sample(t, v, Vec2::ZERO)
			} else {
				controller.sample(t, Vec2::ZERO, v)
			};
		}
		last
	}

	#[test]
	fn a_fast_throw_on_either_stick_is_a_flick() {
		let mut controller = SkillMapController::default();
		let moved = throw(&mut controller, 0.0, Vec2::X, true).expect("move flick");
		assert!(moved.x > 0.9);
		let looked = throw(&mut controller, 1.0, Vec2::Y, false).expect("look flick");
		assert!(looked.y > 0.9);
	}

	#[test]
	fn a_held_stick_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.3, Vec2::X, Vec2::ZERO).is_none());
		assert!(controller.sample(0.35, Vec2::ZERO, Vec2::ZERO).is_none());
	}

	#[test]
	fn rb_is_the_steer_hold() {
		let mut pad = VirtualPad::default();
		assert!(!skill_map_steer_held(&pad));
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		assert!(skill_map_steer_held(&pad));
	}

	#[test]
	fn bumper_chord_is_not_the_steer_hold() {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFocus);
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		assert!(!skill_map_steer_held(&pad));
	}

	#[test]
	fn a_nudge_is_not_a_flick() {
		let mut controller = SkillMapController::default();
		assert!(controller.sample(0.0, Vec2::X * 0.15, Vec2::ZERO).is_none());
		assert!(controller.sample(0.08, Vec2::X * 0.2, Vec2::ZERO).is_none());
		assert!(controller.sample(0.12, Vec2::ZERO, Vec2::ZERO).is_none());
	}
}
