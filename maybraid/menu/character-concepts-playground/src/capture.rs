//! Headless tattoo captures. Active only when `MAYBRAID_TATTOO_CAPTURE_DIR` is set.
//!
//! Uses Bevy `Screenshot` (GPU readback). X11 `scrot` stays black on xvfb.
//!
//! Environment:
//! - `MAYBRAID_TATTOO_CAPTURE_LIGHT` — `day` (default) or `night`
//! - `MAYBRAID_TATTOO_CAPTURE_MOTION` — when set to `walk`, preview uses walk animation
//! - `MAYBRAID_TATTOO_CAPTURE_VIDEO` — when set, records a PNG sequence under
//!   `<capture_dir>/motion_frames/` and writes `<capture_dir>/epiphant_tattoos_motion.mp4` via ffmpeg

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use characters_playground::camera::CameraController;

use crate::camera_focus::PendingCameraFocus;
use crate::preview::{ConceptPreviewConfig, ConceptPreviewRoot, PreviewAwaitingReveal};

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

fn light_suffix() -> &'static str {
	if std::env::var("MAYBRAID_TATTOO_CAPTURE_LIGHT")
		.ok()
		.is_some_and(|s| s.eq_ignore_ascii_case("night"))
	{
		"night"
	} else {
		"day"
	}
}

fn shot_file_name(base: &str) -> String {
	format!("{}_{}.png", base.trim_end_matches(".png"), light_suffix())
}

fn build_shots() -> Vec<(String, Shot)> {
	let shots = [
		Shot {
			name: "warmup",
			eye: Vec3::new(2.2, 0.85, 2.9),
			target: Vec3::new(0.0, 0.70, 0.70),
		},
		Shot {
			name: "whole_body",
			eye: Vec3::new(3.6, 1.35, 2.2),
			target: Vec3::new(0.0, 0.55, 0.15),
		},
		Shot {
			name: "forehead",
			eye: Vec3::new(0.0, 0.42, 2.85),
			target: Vec3::new(0.0, 0.56, 0.75),
		},
		Shot {
			name: "hips",
			eye: Vec3::new(2.9, 0.72, -0.55),
			target: Vec3::new(0.0, 0.48, -0.45),
		},
		Shot {
			name: "body_side",
			eye: Vec3::new(4.0, 1.1, 0.35),
			target: Vec3::new(0.0, 0.55, 0.1),
		},
	];
	shots
		.into_iter()
		.map(|shot| {
			let name = format!("epiphant_tattoos_{}", shot.name);
			(shot_file_name(&name), shot)
		})
		.collect()
}

#[derive(Resource, Default)]
struct TattooCaptureState {
	revealed_at: Option<Duration>,
	index: usize,
	posed_at: Option<Duration>,
	shot: bool,
	shots: Vec<(String, Shot)>,
	video_done: bool,
	video_frame: u32,
	video_next_at: Option<Duration>,
	video_pending_path: Option<PathBuf>,
}

pub fn install(app: &mut App) {
	let Some(dir) = capture_dir() else {
		return;
	};
	let _ = std::fs::create_dir_all(&dir);
	if std::env::var_os("MAYBRAID_TATTOO_CAPTURE_VIDEO").is_some() {
		let _ = std::fs::create_dir_all(dir.join("motion_frames"));
	}
	app.insert_resource(TattooCaptureDir(dir))
		.init_resource::<TattooCaptureState>()
		.add_systems(Startup, apply_capture_lighting)
		.add_systems(
			Update,
			(
				drive_tattoo_captures,
				drive_motion_video,
				quit_after_tattoo_captures,
			)
				.chain(),
		);
}

fn apply_capture_lighting(mut commands: Commands) {
	let night = light_suffix() == "night";
	commands.spawn((
		DirectionalLight {
			illuminance: if night { 120.0 } else { 10000.0 },
			shadow_maps_enabled: !night,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(
			EulerRot::XYZ,
			-if night { 0.15 } else { std::f32::consts::PI / 4.0 },
			if night { 0.4 } else { std::f32::consts::PI / 4.0 },
			0.0,
		)),
		Name::new(if night {
			"tattoo_capture_key_night"
		} else {
			"tattoo_capture_key_day"
		}),
	));
	if !night {
		commands.spawn((
			DirectionalLight { illuminance: 500.0, shadow_maps_enabled: false, ..default() },
			Transform::from_rotation(Quat::from_euler(
				EulerRot::XYZ,
				std::f32::consts::PI / 4.0,
				-std::f32::consts::PI / 4.0,
				0.0,
			)),
			Name::new("tattoo_capture_fill_day"),
		));
	}
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
	if state.shots.is_empty() {
		state.shots = build_shots();
	}
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
		info!("tattoo capture: preview revealed ({})", light_suffix());
	}
	let Some(revealed_at) = state.revealed_at else {
		return;
	};
	if time.elapsed().saturating_sub(revealed_at) < Duration::from_millis(1500) {
		return;
	}
	if state.index >= state.shots.len() {
		return;
	}
	let (filename, shot) = state.shots[state.index].clone();
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
		request_shot(&mut commands, dir.0.join(&filename));
		state.shot = true;
		return;
	}
	if state.shot && dir.0.join(&filename).is_file() {
		state.index += 1;
		state.posed_at = None;
		state.shot = false;
	}
}

const VIDEO_FRAMES: u32 = 48;
const VIDEO_INTERVAL_MS: u64 = 83;

fn drive_motion_video(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<TattooCaptureDir>,
	mut state: ResMut<TattooCaptureState>,
	preview: Query<&Visibility, With<ConceptPreviewRoot>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if std::env::var_os("MAYBRAID_TATTOO_CAPTURE_VIDEO").is_none() {
		return;
	}
	if state.video_done || preview.is_empty() {
		return;
	}
	if preview.iter().all(|visibility| matches!(visibility, Visibility::Hidden)) {
		return;
	}
	if state.revealed_at.is_none() {
		return;
	}
	if state.index < state.shots.len() {
		return;
	}
	if let Ok((mut transform, mut controller)) = cameras.single_mut() {
		characters_playground::camera::orient_camera(
			&mut transform,
			&mut controller,
			Vec3::new(3.8, 1.2, 1.6),
			Vec3::new(0.0, 0.55, 0.2),
		);
	}
	if state.video_next_at.is_none() {
		state.video_next_at = Some(time.elapsed() + Duration::from_millis(800));
		return;
	}
	if time.elapsed() < state.video_next_at.unwrap() {
		return;
	}
	if state.video_frame >= VIDEO_FRAMES {
		encode_motion_video(&dir.0);
		state.video_done = true;
		return;
	}
	if let Some(path) = state.video_pending_path.clone() {
		if path.is_file() {
			state.video_frame += 1;
			state.video_pending_path = None;
			state.video_next_at = Some(time.elapsed() + Duration::from_millis(VIDEO_INTERVAL_MS));
		}
		return;
	}
	let path = dir
		.0
		.join("motion_frames")
		.join(format!("frame_{:04}.png", state.video_frame));
	request_shot(&mut commands, path.clone());
	state.video_pending_path = Some(path);
}

fn encode_motion_video(dir: &Path) {
	let out = dir.join("epiphant_tattoos_motion.mp4");
	let pattern = dir.join("motion_frames/frame_%04d.png");
	let status = Command::new("ffmpeg")
		.args([
			"-y",
			"-framerate",
			"12",
			"-i",
			pattern.to_str().unwrap_or(""),
			"-c:v",
			"libx264",
			"-pix_fmt",
			"yuv420p",
			out.to_str().unwrap_or(""),
		])
		.status();
	match status {
		Ok(s) if s.success() => info!("tattoo capture video {}", out.display()),
		Ok(s) => warn!("ffmpeg exited with {:?}", s.code()),
		Err(e) => warn!("ffmpeg unavailable: {e}"),
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
	let stills_ready = state.index >= state.shots.len()
		&& state
			.shots
			.iter()
			.all(|(name, _)| dir.0.join(name).is_file());
	let video_wanted = std::env::var_os("MAYBRAID_TATTOO_CAPTURE_VIDEO").is_some();
	let video_ready = !video_wanted
		|| (state.video_done && dir.0.join("epiphant_tattoos_motion.mp4").is_file());
	let files_ready = stills_ready && video_ready;
	if files_ready && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(400))
		|| time.elapsed() > Duration::from_secs(180)
	{
		exit.write(AppExit::Success);
	}
}
