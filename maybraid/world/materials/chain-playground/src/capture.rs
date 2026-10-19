//! Validation captures. Video uses deterministic 30 Hz simulation time.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::time::TimeUpdateStrategy;
use chain_shaders::MythicalChainState;
use game_commands::ui::DebugHudRoot;

use crate::preview::{LightingMode, PendingChainHit, PreviewConfig, PreviewStyle};

pub const CAPTURE_CADENCE_HZ: f32 = 30.0;
pub const CAPTURE_DT: f32 = 1.0 / CAPTURE_CADENCE_HZ;

const WARMUP: Duration = Duration::from_millis(12_000);
const SETTLE: Duration = Duration::from_millis(900);

#[derive(Resource, Default)]
pub struct CaptureSet {
	pub dir: Option<PathBuf>,
	step: u32,
	age: f32,
}

impl CaptureSet {
	pub fn start(dir: PathBuf) -> Self {
		Self { dir: Some(dir), step: 0, age: 0.0 }
	}
}

pub fn tick_capture(
	time: Res<Time>,
	mut capture: ResMut<CaptureSet>,
	mut preview: ResMut<PreviewConfig>,
	mut lighting: ResMut<LightingMode>,
	mut commands: Commands,
	mut exits: MessageWriter<AppExit>,
) {
	let Some(dir) = capture.dir.clone() else {
		return;
	};
	capture.age += time.delta_secs();
	let wait = if capture.step == 0 { WARMUP } else { SETTLE };
	if capture.age < wait.as_secs_f32() {
		return;
	}
	capture.age = 0.0;

	match capture.step {
		0 => {
			let _ = std::fs::create_dir_all(&dir);
			preview.halo = true;
			preview.emission = true;
			preview.set_left_damage(0.0);
			preview.set_right_state(MythicalChainState::Intact);
			shoot(&mut commands, dir.join("damage_00_day.png"));
			capture.step = 1;
		}
		1 => {
			preview.set_left_damage(0.25);
			shoot(&mut commands, dir.join("damage_25_day.png"));
			capture.step = 2;
		}
		2 => {
			preview.set_left_damage(0.50);
			shoot(&mut commands, dir.join("damage_50_day.png"));
			capture.step = 3;
		}
		3 => {
			preview.set_left_damage(0.75);
			shoot(&mut commands, dir.join("damage_75_day.png"));
			capture.step = 4;
		}
		4 => {
			preview.set_left_damage(1.0);
			shoot(&mut commands, dir.join("damage_100_day.png"));
			preview.halo = false;
			preview.emission = false;
			preview.restamp_glow_and_seed();
			capture.step = 5;
		}
		5 => {
			preview.set_left_damage(0.0);
			shoot(&mut commands, dir.join("damage_00_day_no_emit.png"));
			capture.step = 6;
		}
		6 => {
			preview.set_left_damage(0.50);
			shoot(&mut commands, dir.join("damage_50_day_no_emit.png"));
			capture.step = 7;
		}
		7 => {
			preview.set_left_damage(1.0);
			shoot(&mut commands, dir.join("damage_100_day_no_emit.png"));
			preview.halo = true;
			preview.emission = true;
			preview.restamp_glow_and_seed();
			*lighting = LightingMode::Night;
			preview.lighting = LightingMode::Night;
			preview.set_left_state(MythicalChainState::Intact);
			preview.set_right_state(MythicalChainState::Damaged);
			capture.step = 8;
		}
		8 => {
			shoot(&mut commands, dir.join("pair_night.png"));
			*lighting = LightingMode::Day;
			preview.lighting = LightingMode::Day;
			preview.style = PreviewStyle::Sharp;
			capture.step = 9;
		}
		9 => {
			shoot(&mut commands, dir.join("pair_day_sharp.png"));
			preview.style = PreviewStyle::Retro;
			capture.step = 10;
		}
		10 => {
			shoot(&mut commands, dir.join("pair_day_retro.png"));
			preview.style = PreviewStyle::Antediluvian;
			capture.step = 11;
		}
		11 => {
			shoot(&mut commands, dir.join("pair_day_antediluvian.png"));
			preview.style = PreviewStyle::Native;
			preview.set_left_state(MythicalChainState::Released);
			capture.step = 12;
		}
		12 => {
			shoot(&mut commands, dir.join("released_day.png"));
			exits.write(AppExit::Success);
			capture.dir = None;
		}
		_ => {
			exits.write(AppExit::Success);
			capture.dir = None;
		}
	}
}

/// Gameplay-distance traveling energy, hits, and style stills. Frames are
/// written at [`CAPTURE_CADENCE_HZ`] with matching simulation `dt`.
#[derive(Resource, Default)]
pub struct VideoCapture {
	pub dir: Option<PathBuf>,
	age: f32,
	warmed: bool,
	manual: bool,
	frame: i32,
	hit_step: u32,
	last_hit_sim: f32,
	done: bool,
}

impl VideoCapture {
	pub fn start(dir: PathBuf) -> Self {
		Self { dir: Some(dir), ..default() }
	}
}

pub fn hide_hud_during_evidence(
	video: Res<VideoCapture>,
	capture: Res<CaptureSet>,
	mut hud: Query<&mut Visibility, With<DebugHudRoot>>,
) {
	if video.dir.is_none() && capture.dir.is_none() {
		return;
	}
	for mut vis in &mut hud {
		*vis = Visibility::Hidden;
	}
}

pub fn tick_video(
	time: Res<Time>,
	mut video: ResMut<VideoCapture>,
	mut preview: ResMut<PreviewConfig>,
	mut lighting: ResMut<LightingMode>,
	mut pending: ResMut<PendingChainHit>,
	mut commands: Commands,
	mut exits: MessageWriter<AppExit>,
) {
	let Some(dir) = video.dir.clone() else {
		return;
	};
	if !video.warmed {
		if video.age == 0.0 {
			let _ = std::fs::create_dir_all(dir.join("frames"));
			*lighting = LightingMode::Day;
			preview.lighting = LightingMode::Day;
			preview.style = PreviewStyle::Native;
			preview.halo = true;
			preview.emission = true;
			preview.deform = true;
			preview.set_left_state(MythicalChainState::Intact);
			preview.set_right_state(MythicalChainState::Damaged);
		}
		video.age += time.delta_secs();
		if video.age < WARMUP.as_secs_f32() {
			return;
		}
		video.warmed = true;
		video.age = 0.0;
		commands.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
			CAPTURE_DT,
		)));
		video.manual = true;
		info!(
			"chain video capture cadence {} Hz dt={CAPTURE_DT:.4}s (deterministic virtual time)",
			CAPTURE_CADENCE_HZ
		);
		return;
	}

	video.age += CAPTURE_DT;
	let sim = video.age;

	// Repeated small hits on one link, waiting for the transient to settle.
	if video.hit_step < 3 {
		let due = 1.2 + video.hit_step as f32 * 0.85;
		if sim >= due && sim - video.last_hit_sim > 0.80 {
			*pending = PendingChainHit { link: Some(6), amount: 0.22 };
			video.last_hit_sim = sim;
			video.hit_step += 1;
		}
	} else if video.hit_step == 3 && sim >= 4.2 {
		*pending = PendingChainHit { link: Some(3), amount: 0.35 };
		video.hit_step = 4;
	} else if video.hit_step == 4 && sim >= 5.1 {
		*pending = PendingChainHit { link: Some(10), amount: 0.35 };
		video.hit_step = 5;
	}

	if (10.2..10.3).contains(&sim) {
		shoot(&mut commands, dir.join("flow_transit.png"));
	}
	if (11.2..11.3).contains(&sim) {
		preview.style = PreviewStyle::Retro;
	}
	if (11.6..11.7).contains(&sim) {
		shoot(&mut commands, dir.join("flow_retro.png"));
	}
	if (12.0..12.1).contains(&sim) {
		preview.style = PreviewStyle::Antediluvian;
	}
	if (12.4..12.5).contains(&sim) {
		shoot(&mut commands, dir.join("flow_antediluvian.png"));
	}

	if (0.0..10.2).contains(&sim) && !video.done {
		shoot(&mut commands, dir.join("frames").join(format!("f{:04}.png", video.frame)));
		video.frame += 1;
	}

	if sim > 13.0 && !video.done {
		video.done = true;
		exits.write(AppExit::Success);
		video.dir = None;
	}
}

fn shoot(commands: &mut Commands, path: PathBuf) {
	info!("capturing {}", path.display());
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}
