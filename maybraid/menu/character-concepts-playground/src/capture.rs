//! Headless tattoo captures. Active only when `MAYBRAID_TATTOO_CAPTURE_DIR` is set.
//!
//! Uses Bevy `Screenshot` (GPU readback). X11 `scrot` stays black on xvfb.
//! After preview reveal, shoots head front / three-quarter / side, then a
//! full-body side view.

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use characters_playground::camera::CameraController;

use crate::camera_focus::PendingCameraFocus;
use crate::preview::{ConceptPreviewRoot, PreviewAwaitingReveal};

fn capture_dir() -> Option<PathBuf> {
	std::env::var_os("MAYBRAID_TATTOO_CAPTURE_DIR").map(PathBuf::from)
}

#[derive(Resource)]
struct TattooCaptureDir(PathBuf);

#[derive(Clone, Copy)]
struct Shot {
	name: &'static str,
	eye: Vec3,
	target: Vec3,
}

const SHOTS: [Shot; 5] = [
	// First GPU readback is a stale frame; discard it.
	Shot {
		name: "epiphant_tattoos_warmup.png",
		eye: Vec3::new(2.2, 0.85, 2.9),
		target: Vec3::new(0.0, 0.70, 0.70),
	},
	Shot {
		name: "epiphant_tattoos_head_front.png",
		eye: Vec3::new(0.0, 0.42, 2.85),
		target: Vec3::new(0.0, 0.56, 0.75),
	},
	Shot {
		name: "epiphant_tattoos_head_three_quarter.png",
		eye: Vec3::new(2.2, 0.85, 2.9),
		target: Vec3::new(0.0, 0.70, 0.70),
	},
	Shot {
		name: "epiphant_tattoos_head_side.png",
		eye: Vec3::new(3.2, 0.75, 0.90),
		target: Vec3::new(0.0, 0.68, 0.75),
	},
	Shot {
		name: "epiphant_tattoos_body_side.png",
		eye: Vec3::new(4.0, 1.1, 0.35),
		target: Vec3::new(0.0, 0.55, 0.1),
	},
];

#[derive(Resource, Default)]
struct TattooCaptureState {
	revealed_at: Option<Duration>,
	index: usize,
	posed_at: Option<Duration>,
	shot: bool,
}

pub fn install(app: &mut App) {
	let Some(dir) = capture_dir() else {
		return;
	};
	let _ = std::fs::create_dir_all(&dir);
	app.insert_resource(TattooCaptureDir(dir))
		.init_resource::<TattooCaptureState>()
		.add_systems(Update, (drive_tattoo_captures, quit_after_tattoo_captures).chain());
}

fn drive_tattoo_captures(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<TattooCaptureDir>,
	mut state: ResMut<TattooCaptureState>,
	pending_reveal: Query<(), With<PreviewAwaitingReveal>>,
	preview: Query<&Visibility, With<ConceptPreviewRoot>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
	mut pending_focus: ResMut<PendingCameraFocus>,
) {
	if preview.is_empty() || !pending_reveal.is_empty() {
		return;
	}
	if preview.iter().all(|visibility| matches!(visibility, Visibility::Hidden)) {
		return;
	}
	if state.revealed_at.is_none() {
		state.revealed_at = Some(time.elapsed());
		pending_focus.focus = None;
		pending_focus.focus_trigger = None;
		pending_focus.resolved_target = None;
		info!("tattoo capture: preview revealed");
	}
	let Some(revealed_at) = state.revealed_at else {
		return;
	};
	if time.elapsed().saturating_sub(revealed_at) < Duration::from_millis(1500) {
		return;
	}
	if state.index >= SHOTS.len() {
		return;
	}
	let shot = SHOTS[state.index];
	pending_focus.focus = None;
	pending_focus.focus_trigger = None;
	pending_focus.resolved_target = None;
	if let Ok((mut transform, mut controller)) = cameras.single_mut() {
		characters_playground::camera::orient_camera(
			&mut transform,
			&mut controller,
			shot.eye,
			shot.target,
		);
	}
	if state.posed_at.is_none() {
		state.posed_at = Some(time.elapsed());
		state.shot = false;
		return;
	}
	let Some(posed_at) = state.posed_at else {
		return;
	};
	if !state.shot && time.elapsed().saturating_sub(posed_at) >= Duration::from_millis(450) {
		request_shot(&mut commands, dir.0.join(shot.name));
		state.shot = true;
		return;
	}
	if state.shot && dir.0.join(shot.name).is_file() {
		state.index += 1;
		state.posed_at = None;
		state.shot = false;
	}
}

fn request_shot(commands: &mut Commands, path: impl AsRef<Path>) {
	let path = path.as_ref().to_owned();
	info!("tattoo capture {}", path.display());
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}

fn quit_after_tattoo_captures(
	time: Res<Time>,
	dir: Res<TattooCaptureDir>,
	state: Res<TattooCaptureState>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
) {
	let files_ready =
		state.index >= SHOTS.len() && SHOTS.iter().all(|shot| dir.0.join(shot.name).is_file());
	if files_ready && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(400))
		|| time.elapsed() > Duration::from_secs(120)
	{
		exit.write(AppExit::Success);
	}
}
