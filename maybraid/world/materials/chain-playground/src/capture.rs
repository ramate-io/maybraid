//! Headless validation captures for the four authored states, day/night, and a mixed pair.

use std::path::PathBuf;
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use chain_shaders::MythicalChainState;

use crate::preview::{LightingMode, PreviewConfig};

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

fn shoot(commands: &mut Commands, path: PathBuf) {
	info!("capturing {}", path.display());
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}
