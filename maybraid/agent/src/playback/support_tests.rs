//! Movement commands wait for the selected player's bounded spawn handoff.
use crate::command::{AgentButton, AgentCommand};
use crate::pad::PadIntent;
use crate::playback::{Playback, Tick, TickEffect, TickView};

fn goto(timeout: f32) -> AgentCommand {
	AgentCommand::Goto {
		xz: [100.0, 0.0],
		timeout,
		radius: 3.0,
		sprint: true,
		look: Some([0.3, 0.2]),
	}
}

fn view(x: f32, waiting: bool) -> TickView {
	TickView {
		player_xz: Some([x, 0.0]),
		player_waiting_for_support: waiting,
		..Default::default()
	}
}

fn failure(tick: &Tick) -> Option<&str> {
	tick.effects.iter().find_map(|effect| match effect {
		TickEffect::Failed { detail, .. } => Some(detail.as_str()),
		_ => None,
	})
}

#[test]
fn pending_goto_stays_neutral_beyond_stuck_limit_and_cannot_reach_held_target() {
	let mut playback = Playback::from_commands([goto(20.0)]);
	let waiting = playback.tick(6.0, &view(100.0, true));
	assert_eq!(waiting.intent, PadIntent::default());
	assert_eq!(failure(&waiting), None);
	assert!(!waiting.idle);
	assert!(!waiting
		.effects
		.iter()
		.any(|effect| matches!(effect, TickEffect::Waypoint { reached: true, .. })));
	let released = playback.tick(0.1, &view(100.0, false));
	assert!(released.idle);
	assert!(released
		.effects
		.iter()
		.any(|effect| matches!(effect, TickEffect::Waypoint { reached: true, .. })));
}

#[test]
fn pending_transition_resets_stall_without_erasing_movement_timeout() {
	let mut stalled = Playback::from_commands([goto(30.0)]);
	stalled.tick(0.1, &view(0.0, false));
	stalled.tick(4.0, &view(0.0, false));
	let paused = stalled.tick(1.0, &view(0.0, true));
	assert_eq!(paused.intent, PadIntent::default());
	stalled.tick(0.1, &view(0.0, false));
	let resumed = stalled.tick(1.0, &view(0.0, false));
	assert!(!resumed.intent.buttons.contains(&AgentButton::Jump));
	assert_eq!(failure(&resumed), None);

	let mut timed = Playback::from_commands([goto(3.0)]);
	timed.tick(1.0, &view(0.0, false));
	timed.tick(2.0, &view(0.0, true));
	assert_eq!(failure(&timed.tick(1.5, &view(10.0, false))), None);
	assert_eq!(failure(&timed.tick(0.6, &view(20.0, false))), Some("timeout"));
}

#[test]
fn support_wait_timeout_is_bounded_across_readiness_flaps() {
	let mut persistent = Playback::from_commands([goto(4.0)]);
	let timeout = persistent.tick(4.0, &view(0.0, true));
	assert_eq!(failure(&timeout), Some("support timeout"));
	assert_eq!(timeout.intent, PadIntent::default());
	assert!(timeout.idle);

	let mut flapping = Playback::from_commands([goto(4.0)]);
	assert_eq!(failure(&flapping.tick(2.2, &view(0.0, true))), None);
	assert_eq!(failure(&flapping.tick(0.1, &view(0.0, false))), None);
	assert_eq!(failure(&flapping.tick(1.9, &view(0.0, true))), Some("support timeout"));
}

#[test]
fn missing_player_keeps_existing_movement_timeout_and_wait_hold_keep_ticking() {
	for waiting in [false, true] {
		let mut playback = Playback::from_commands([goto(3.0)]);
		let missing = TickView { player_waiting_for_support: waiting, ..Default::default() };
		assert_eq!(failure(&playback.tick(3.1, &missing)), Some("timeout"));
	}
	let mut wait = Playback::from_commands([AgentCommand::Wait { secs: 1.0 }]);
	assert!(wait.tick(1.1, &view(0.0, true)).idle);
	let mut hold = Playback::from_commands([AgentCommand::Hold {
		r#move: Some([0.0, 1.0]),
		look: None,
		focus: None,
		fire: None,
		buttons: Vec::new(),
		secs: 1.0,
	}]);
	let held = hold.tick(1.1, &view(0.0, true));
	assert!(held.idle);
	assert_eq!(held.intent.move_stick, [0.0, 1.0]);
}

#[test]
fn start_and_teleport_barriers_prevent_goto_using_the_old_pose() {
	for command in [
		AgentCommand::Start { xz: [200.0, 0.0] },
		AgentCommand::Teleport { xz: [200.0, 0.0], y: None },
	] {
		let mut playback = Playback::from_commands([command, goto(30.0)]);
		let relocating = playback.tick(10.0, &view(100.0, false));
		assert_eq!(relocating.intent, PadIntent::default());
		assert!(!relocating.idle);
		assert!(!relocating.effects.iter().any(|effect| matches!(
			effect,
			TickEffect::Started { id: 1, .. } | TickEffect::Waypoint { .. }
		)));
		let next = playback.tick(0.1, &view(200.0, true));
		assert_eq!(next.intent, PadIntent::default());
		assert!(!next.idle);
		assert!(next
			.effects
			.iter()
			.any(|effect| matches!(effect, TickEffect::Started { id: 1, .. })));
	}
}
