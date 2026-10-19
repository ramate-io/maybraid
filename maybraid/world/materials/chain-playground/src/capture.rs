//! Headless validation captures: authored stills plus a gameplay-distance video sequence.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use chain_shaders::MythicalChainState;

use crate::preview::{LightingMode, PendingChainHit, PreviewConfig, PreviewStyle};

const WARMUP: Duration = Duration::from_millis(2200);
const SETTLE: Duration = Duration::from_millis(750);
const QUIT_AFTER: Duration = Duration::from_millis(400);

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
			shoot(&mut commands, dir.join("mythical_chain_intact_damaged_day.png"));
			*lighting = LightingMode::Night;
			preview.lighting = LightingMode::Night;
			capture.step = 1;
		}
		1 => {
			shoot(&mut commands, dir.join("mythical_chain_intact_damaged_night.png"));
			*lighting = LightingMode::Day;
			preview.lighting = LightingMode::Day;
			preview.set_left_state(MythicalChainState::NearBreaking);
			preview.set_right_state(MythicalChainState::NearBreaking);
			capture.step = 2;
		}
		2 => {
			shoot(&mut commands, dir.join("mythical_chain_near_breaking_day.png"));
			preview.set_left_state(MythicalChainState::Released);
			preview.set_right_state(MythicalChainState::Released);
			capture.step = 3;
		}
		3 => {
			shoot(&mut commands, dir.join("mythical_chain_released_day.png"));
			preview.set_left_state(MythicalChainState::NearBreaking);
			preview.set_right_state(MythicalChainState::Released);
			capture.step = 4;
		}
		4 => {
			shoot(&mut commands, dir.join("mythical_chain_near_breaking_and_released.png"));
			capture.step = 5;
		}
		_ => {
			if capture.age + wait.as_secs_f32() >= QUIT_AFTER.as_secs_f32() {
				exits.write(AppExit::Success);
				capture.dir = None;
			}
		}
	}
}

/// Gameplay-distance localized hit + several pulse cycles. Stills are written
/// here; the host script records the window with ffmpeg.
#[derive(Resource, Default)]
pub struct VideoCapture {
	pub dir: Option<PathBuf>,
	age: f32,
	hit: bool,
	day_still: bool,
	hit_still: bool,
	crest_still: bool,
	retro_still: bool,
	done: bool,
	last_frame: i32,
}

impl VideoCapture {
	pub fn start(dir: PathBuf) -> Self {
		Self { dir: Some(dir), ..default() }
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
	if video.age == 0.0 && !video.day_still {
		let _ = std::fs::create_dir_all(dir.join("frames"));
		*lighting = LightingMode::Day;
		preview.lighting = LightingMode::Day;
		preview.style = PreviewStyle::Sharp;
		preview.set_left_state(MythicalChainState::Intact);
		preview.set_right_state(MythicalChainState::Damaged);
	}
	video.age += time.delta_secs();

	// Lavapipe shader compile is slow; wait before evidence stills.
	if video.age > 14.0 && !video.day_still {
		shoot(&mut commands, dir.join("gameplay_day_intact.png"));
		video.day_still = true;
	}
	if video.age > 14.6 && !video.hit {
		*pending = PendingChainHit { link: Some(6), amount: 0.78 };
		video.hit = true;
	}
	if video.age > 15.1 && !video.hit_still {
		shoot(&mut commands, dir.join("gameplay_day_hit.png"));
		video.hit_still = true;
	}
	if video.age > 16.4 && !video.crest_still {
		shoot(&mut commands, dir.join("gameplay_day_pulse.png"));
		video.crest_still = true;
	}
	if video.age > 18.2 && preview.style != PreviewStyle::Retro && !video.retro_still {
		preview.style = PreviewStyle::Retro;
	}
	if video.age > 18.9 && !video.retro_still {
		shoot(&mut commands, dir.join("gameplay_retro_hit.png"));
		video.retro_still = true;
	}
	if video.age > 20.2 && preview.style != PreviewStyle::Antediluvian && !video.done {
		preview.style = PreviewStyle::Antediluvian;
	}
	if video.age > 21.4 && !video.done {
		shoot(&mut commands, dir.join("gameplay_antediluvian_hit.png"));
		video.done = true;
	}
	if video.age > 14.0 && video.age < 18.0 {
		let frame = ((video.age - 14.0) * 5.0).floor() as i32;
		if frame != video.last_frame {
			video.last_frame = frame;
			shoot(
				&mut commands,
				dir.join("frames").join(format!("f{frame:04}.png")),
			);
		}
	}
	if video.age > 22.2 {
		exits.write(AppExit::Success);
		video.dir = None;
	}
}

fn shoot(commands: &mut Commands, path: PathBuf) {
	info!("capturing {}", path.display());
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}
