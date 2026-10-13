//! Headless tattoo captures. Active only when `MAYBRAID_TATTOO_CAPTURE_DIR` is set.
//!
//! Uses Bevy `Screenshot` (GPU readback). X11 `scrot` stays black on xvfb.
//! Waits for preview reveal and species camera focus to settle, then shoots
//! the default framing and a yawed side view one frame after each camera move.

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

#[derive(Resource, Default)]
struct TattooCaptureState {
	revealed_at: Option<Duration>,
	saw_focus: bool,
	focus_settled: bool,
	default_shot: bool,
	side_posed_at: Option<Duration>,
	side_shot: bool,
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
	focus: Res<PendingCameraFocus>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if preview.is_empty() || !pending_reveal.is_empty() {
		return;
	}
	if preview.iter().all(|visibility| matches!(visibility, Visibility::Hidden)) {
		return;
	}
	if state.revealed_at.is_none() {
		state.revealed_at = Some(time.elapsed());
		info!("tattoo capture: preview revealed");
	}
	let Some(revealed_at) = state.revealed_at else {
		return;
	};
	let elapsed = time.elapsed().saturating_sub(revealed_at);

	if focus.focus.is_some() {
		state.saw_focus = true;
	}
	if state.saw_focus && focus.focus.is_none() {
		state.focus_settled = true;
	}
	let framing_ready = (state.focus_settled && elapsed >= Duration::from_millis(400))
		|| elapsed >= Duration::from_secs(4);

	if framing_ready && !state.default_shot {
		request_shot(&mut commands, dir.0.join("epiphant_tattoos_three_quarter.png"));
		state.default_shot = true;
		return;
	}

	if state.default_shot
		&& state.side_posed_at.is_none()
		&& dir.0.join("epiphant_tattoos_three_quarter.png").is_file()
	{
		if let Ok((mut transform, mut controller)) = cameras.single_mut() {
			let target = transform.translation + transform.forward() * 3.0;
			let offset = transform.right() * 3.4 + Vec3::Y * 0.15;
			characters_playground::camera::orient_camera(
				&mut transform,
				&mut controller,
				target + offset,
				target,
			);
		}
		state.side_posed_at = Some(time.elapsed());
		return;
	}

	if let Some(posed_at) = state.side_posed_at {
		if !state.side_shot && time.elapsed().saturating_sub(posed_at) >= Duration::from_millis(400)
		{
			request_shot(&mut commands, dir.0.join("epiphant_tattoos_side.png"));
			state.side_shot = true;
		}
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
	let files_ready = state.default_shot
		&& state.side_shot
		&& dir.0.join("epiphant_tattoos_side.png").is_file()
		&& dir.0.join("epiphant_tattoos_three_quarter.png").is_file();
	if files_ready && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(400))
		|| time.elapsed() > Duration::from_secs(120)
	{
		exit.write(AppExit::Success);
	}
}
