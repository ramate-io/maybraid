//! Validation captures. Video uses deterministic 30 Hz simulation time.
//!
//! Stills apply a look, wait for `sync_preview` + a GPU frame, then shoot.
//! They never change state in the same arm as the screenshot, and they wait
//! after the last shot so Bevy can write the file before `AppExit`.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::time::TimeUpdateStrategy;
use chain_shaders::MythicalChainState;
use game_commands::ui::DebugHudRoot;

use crate::preview::{LightingMode, PendingChainHit, PreviewConfig, PreviewStyle};

pub const CAPTURE_CADENCE_HZ: f32 = 30.0;
pub const CAPTURE_DT: f32 = 1.0 / CAPTURE_CADENCE_HZ;

const WARMUP: Duration = Duration::from_millis(12_000);
const SETTLE: Duration = Duration::from_millis(700);

#[derive(Clone, Copy)]
enum StillKind {
	Damage {
		amount: f32,
		emit: bool,
	},
	Pair {
		lighting: LightingMode,
		style: PreviewStyle,
		left: MythicalChainState,
		right: MythicalChainState,
	},
	Released,
}

struct StillShot {
	file: &'static str,
	kind: StillKind,
}

const STILLS: &[StillShot] = &[
	StillShot { file: "damage_00_day.png", kind: StillKind::Damage { amount: 0.0, emit: true } },
	StillShot { file: "damage_25_day.png", kind: StillKind::Damage { amount: 0.25, emit: true } },
	StillShot { file: "damage_50_day.png", kind: StillKind::Damage { amount: 0.50, emit: true } },
	StillShot { file: "damage_75_day.png", kind: StillKind::Damage { amount: 0.75, emit: true } },
	StillShot { file: "damage_100_day.png", kind: StillKind::Damage { amount: 1.0, emit: true } },
	StillShot {
		file: "damage_00_day_no_emit.png",
		kind: StillKind::Damage { amount: 0.0, emit: false },
	},
	StillShot {
		file: "damage_25_day_no_emit.png",
		kind: StillKind::Damage { amount: 0.25, emit: false },
	},
	StillShot {
		file: "damage_50_day_no_emit.png",
		kind: StillKind::Damage { amount: 0.50, emit: false },
	},
	StillShot {
		file: "damage_75_day_no_emit.png",
		kind: StillKind::Damage { amount: 0.75, emit: false },
	},
	StillShot {
		file: "damage_100_day_no_emit.png",
		kind: StillKind::Damage { amount: 1.0, emit: false },
	},
	StillShot {
		file: "pair_night.png",
		kind: StillKind::Pair {
			lighting: LightingMode::Night,
			style: PreviewStyle::Native,
			left: MythicalChainState::Intact,
			right: MythicalChainState::Damaged,
		},
	},
	StillShot {
		file: "pair_day_sharp.png",
		kind: StillKind::Pair {
			lighting: LightingMode::Day,
			style: PreviewStyle::Sharp,
			left: MythicalChainState::Intact,
			right: MythicalChainState::Damaged,
		},
	},
	StillShot {
		file: "pair_day_retro.png",
		kind: StillKind::Pair {
			lighting: LightingMode::Day,
			style: PreviewStyle::Retro,
			left: MythicalChainState::Intact,
			right: MythicalChainState::Damaged,
		},
	},
	StillShot {
		file: "pair_day_antediluvian.png",
		kind: StillKind::Pair {
			lighting: LightingMode::Day,
			style: PreviewStyle::Antediluvian,
			left: MythicalChainState::Intact,
			right: MythicalChainState::Damaged,
		},
	},
	StillShot { file: "released_day.png", kind: StillKind::Released },
];

#[derive(Resource, Default)]
pub struct CaptureSet {
	pub dir: Option<PathBuf>,
	/// 0 = warmup, then `2*i+1` apply, `2*i+2` shoot, last extra wait.
	step: u32,
	age: f32,
	frozen: bool,
	settle_from: Option<Instant>,
}

impl CaptureSet {
	pub fn start(dir: PathBuf) -> Self {
		Self { dir: Some(dir), step: 0, age: 0.0, frozen: false, settle_from: None }
	}
}

fn apply_still(shot: &StillShot, preview: &mut PreviewConfig, lighting: &mut LightingMode) {
	match shot.kind {
		StillKind::Damage { amount, emit } => {
			*lighting = LightingMode::Day;
			preview.lighting = LightingMode::Day;
			preview.style = PreviewStyle::Native;
			preview.halo = emit;
			preview.emission = emit;
			preview.set_left_damage(amount);
			preview.set_right_state(MythicalChainState::Intact);
			preview.restamp_glow_and_seed();
		}
		StillKind::Pair { lighting: mode, style, left, right } => {
			*lighting = mode;
			preview.lighting = mode;
			preview.style = style;
			preview.halo = true;
			preview.emission = true;
			preview.set_left_state(left);
			preview.set_right_state(right);
			preview.restamp_glow_and_seed();
		}
		StillKind::Released => {
			*lighting = LightingMode::Day;
			preview.lighting = LightingMode::Day;
			preview.style = PreviewStyle::Native;
			preview.halo = true;
			preview.emission = true;
			preview.set_left_state(MythicalChainState::Released);
			preview.set_right_state(MythicalChainState::Intact);
			preview.restamp_glow_and_seed();
		}
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

	if capture.step == 0 {
		capture.age += time.delta_secs();
		if capture.age < 0.05 {
			info!("chain stills warmup started");
		}
		if capture.age < WARMUP.as_secs_f32() {
			return;
		}
		let _ = std::fs::create_dir_all(&dir);
		if !capture.frozen {
			commands.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
			capture.frozen = true;
		}
		apply_still(&STILLS[0], &mut preview, &mut lighting);
		capture.settle_from = Some(Instant::now());
		capture.step = 1;
		info!("chain stills applying {}", STILLS[0].file);
		return;
	}

	// Virtual time is frozen after warmup; settle uses wall time.
	if let Some(from) = capture.settle_from {
		if from.elapsed() < SETTLE {
			return;
		}
		capture.settle_from = None;
	}

	let shot_i = ((capture.step - 1) / 2) as usize;
	let apply_phase = capture.step % 2 == 1;

	if shot_i >= STILLS.len() {
		exits.write(AppExit::Success);
		capture.dir = None;
		return;
	}

	if apply_phase {
		shoot(&mut commands, dir.join(STILLS[shot_i].file));
		capture.settle_from = Some(Instant::now());
		capture.step += 1;
		return;
	}

	let next = shot_i + 1;
	if next >= STILLS.len() {
		capture.settle_from = Some(Instant::now());
		capture.step += 1;
		return;
	}
	apply_still(&STILLS[next], &mut preview, &mut lighting);
	capture.settle_from = Some(Instant::now());
	capture.step += 1;
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

	// 0..10.2 s is the no-hit flow reel. Hits start after that so the
	// 30 Hz mp4 is a fixed-damage transit, not a flash sequence.
	if video.hit_step < 3 {
		let due = 10.35 + video.hit_step as f32 * 0.85;
		if sim >= due && sim - video.last_hit_sim > 0.80 {
			*pending = PendingChainHit { link: Some(6), amount: 0.22 };
			video.last_hit_sim = sim;
			video.hit_step += 1;
		}
	} else if video.hit_step == 3 && sim >= 13.0 {
		*pending = PendingChainHit { link: Some(3), amount: 0.35 };
		video.hit_step = 4;
	} else if video.hit_step == 4 && sim >= 13.9 {
		*pending = PendingChainHit { link: Some(10), amount: 0.35 };
		video.hit_step = 5;
	}

	if (10.15..10.25).contains(&sim) {
		shoot(&mut commands, dir.join("flow_transit.png"));
	}
	if (12.05..12.15).contains(&sim) {
		shoot(&mut commands, dir.join("hit_link6.png"));
	}
	if (13.55..13.65).contains(&sim) {
		preview.style = PreviewStyle::Retro;
	}
	if (13.95..14.05).contains(&sim) {
		shoot(&mut commands, dir.join("flow_retro.png"));
	}
	if (14.35..14.45).contains(&sim) {
		preview.style = PreviewStyle::Antediluvian;
	}
	if (14.75..14.85).contains(&sim) {
		shoot(&mut commands, dir.join("flow_antediluvian.png"));
	}

	if (0.0..10.2).contains(&sim) && !video.done {
		shoot(&mut commands, dir.join("frames").join(format!("f{:04}.png", video.frame)));
		video.frame += 1;
	}

	if sim > 16.0 && !video.done {
		video.done = true;
		exits.write(AppExit::Success);
		video.dir = None;
	}
}

fn shoot(commands: &mut Commands, path: PathBuf) {
	info!("capturing {}", path.display());
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
}
