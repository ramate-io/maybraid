//! Playground-only walk through loading phases after `show loading`.

use bevy::prelude::*;
use menu_screens::{request_loading_explainer, request_loading_progress, LoadingScreen};

pub struct Phase {
	pub explainer: &'static str,
	pub progress: f32,
	pub hold_secs: f32,
}

pub const PHASES: &[Phase] = &[
	Phase { explainer: "Waking the world…", progress: 0.12, hold_secs: 0.9 },
	Phase { explainer: "Tracing the hidden thread…", progress: 0.34, hold_secs: 1.1 },
	Phase { explainer: "Gathering remnants…", progress: 0.58, hold_secs: 1.0 },
	Phase { explainer: "Binding the braid…", progress: 0.82, hold_secs: 1.0 },
	Phase { explainer: "Ready.", progress: 1.0, hold_secs: 0.7 },
];

/// Active demo. `phase_started` is `None` until the loading screen exists.
#[derive(Resource, Debug, Default)]
pub struct LoadingDemo {
	pub phase: usize,
	pub phase_started: Option<f32>,
}

pub fn run_loading_demo(
	time: Res<Time>,
	demo: Option<ResMut<LoadingDemo>>,
	screens: Query<(), With<LoadingScreen>>,
	mut commands: Commands,
) {
	let Some(mut demo) = demo else {
		return;
	};
	if screens.is_empty() {
		if demo.phase_started.is_some() {
			commands.remove_resource::<LoadingDemo>();
		}
		return;
	}
	if demo.phase >= PHASES.len() {
		commands.remove_resource::<LoadingDemo>();
		return;
	}

	let now = time.elapsed_secs();
	let Some(started) = demo.phase_started else {
		apply_phase(&mut commands, demo.phase);
		demo.phase_started = Some(now);
		return;
	};

	if now - started < PHASES[demo.phase].hold_secs {
		return;
	}
	demo.phase += 1;
	if demo.phase >= PHASES.len() {
		commands.remove_resource::<LoadingDemo>();
		return;
	}
	apply_phase(&mut commands, demo.phase);
	demo.phase_started = Some(now);
}

fn apply_phase(commands: &mut Commands, index: usize) {
	let phase = &PHASES[index];
	request_loading_progress(commands, phase.progress);
	request_loading_explainer(commands, phase.explainer);
}
