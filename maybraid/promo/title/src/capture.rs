//! Frame-exact PNG dump, then ffmpeg. Sequence time is the capture clock.

use std::path::{Path, PathBuf};

use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use bevy::time::TimeUpdateStrategy;
use bevy::winit::WinitSettings;

use crate::encode::{
	apply_swallow_alpha, composite_over_checkerboard, encode_overlay_sequence, encode_png_sequence,
};
use crate::plugin::{SequenceClock, TitleSequenceConfig, TitleSequenceSystems};
use crate::timeline::Keyframe;

/// Disk targets for one capture run.
#[derive(Resource, Clone, Debug)]
pub struct CaptureIo {
	pub output: PathBuf,
	pub overlay: PathBuf,
	pub frames_dir: PathBuf,
	pub stills_dir: PathBuf,
	pub ffmpeg: PathBuf,
	pub keep_frames: bool,
	pub warmup_frames: u32,
	pub settle_shots: u32,
}

impl CaptureIo {
	pub fn new(
		output: PathBuf,
		overlay: Option<PathBuf>,
		ffmpeg: PathBuf,
		keep_frames: bool,
		warmup_frames: u32,
	) -> Self {
		let stem = output.file_stem().unwrap_or_else(|| std::ffi::OsStr::new("title-sequence"));
		let parent = output
			.parent()
			.filter(|path| !path.as_os_str().is_empty())
			.unwrap_or_else(|| Path::new("."));
		let overlay =
			overlay.unwrap_or_else(|| parent.join(format!("{}.mov", stem.to_string_lossy())));
		Self {
			frames_dir: parent.join(format!("{}_frames", stem.to_string_lossy())),
			stills_dir: parent.join(format!("{}_stills", stem.to_string_lossy())),
			output,
			overlay,
			ffmpeg,
			keep_frames,
			warmup_frames,
			settle_shots: 8,
		}
	}
}

#[derive(Resource, Debug)]
struct CaptureState {
	stage: Stage,
}

#[derive(Debug)]
enum Stage {
	Warmup { frames: u32 },
	Settle { remaining: u32, requested: bool },
	Shot { frame: u32, requested: bool },
	Encode,
}

/// Drive [`SequenceClock`] from the capture frame index and dump PNGs.
pub struct TitleCapturePlugin;

impl Plugin for TitleCapturePlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::ZERO))
			.insert_resource(WinitSettings::continuous())
			.insert_resource(CaptureState { stage: Stage::Warmup { frames: 0 } })
			.add_systems(
				Update,
				prepare_capture_frame
					.after(TitleSequenceSystems::Clock)
					.before(TitleSequenceSystems::ShaderTime),
			)
			.add_systems(Update, request_capture_frame.after(TitleSequenceSystems::Present));
	}
}

fn prepare_capture_frame(
	config: Res<TitleSequenceConfig>,
	mut clock: ResMut<SequenceClock>,
	state: Res<CaptureState>,
) {
	match state.stage {
		Stage::Warmup { .. } | Stage::Settle { .. } => clock.time = 0.0,
		Stage::Shot { frame, requested: false } => {
			clock.time = config.sequence.time_of_frame(frame, config.fps);
		}
		Stage::Shot { .. } | Stage::Encode => {}
	}
}

fn request_capture_frame(
	mut commands: Commands,
	config: Res<TitleSequenceConfig>,
	io: Res<CaptureIo>,
	mut state: ResMut<CaptureState>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
) {
	if !capturing.is_empty() {
		return;
	}

	match state.stage {
		Stage::Warmup { frames } => {
			if frames + 1 >= io.warmup_frames {
				if let Err(err) = std::fs::create_dir_all(&io.frames_dir) {
					error!("title capture: {err}");
					exit.write(AppExit::from_code(1));
					return;
				}
				info!("title capture: settling render targets");
				state.stage = Stage::Settle { remaining: io.settle_shots.max(1), requested: false };
			} else {
				state.stage = Stage::Warmup { frames: frames + 1 };
			}
		}
		Stage::Settle { remaining, requested } => {
			let path = io.frames_dir.join(".settle.png");
			if !requested {
				commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
				state.stage = Stage::Settle { remaining, requested: true };
				return;
			}
			if !path.is_file() {
				return;
			}
			let _ = std::fs::remove_file(&path);
			if remaining <= 1 {
				info!("title capture: starting {} frames", config.sequence.frame_count(config.fps));
				state.stage = Stage::Shot { frame: 0, requested: false };
			} else {
				state.stage = Stage::Settle { remaining: remaining - 1, requested: false };
			}
		}
		Stage::Shot { frame, requested } => {
			let total = config.sequence.frame_count(config.fps);
			if !requested {
				let path = io.frames_dir.join(format!("frame_{frame:05}.png"));
				commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
				state.stage = Stage::Shot { frame, requested: true };
				return;
			}
			let path = io.frames_dir.join(format!("frame_{frame:05}.png"));
			if !path.is_file() {
				return;
			}
			if frame == 0 || (frame + 1) % 30 == 0 || frame + 1 == total {
				info!("title capture: frame {}/{}", frame + 1, total);
			}
			if frame + 1 >= total {
				state.stage = Stage::Encode;
				if let Err(err) = finish_capture(&config, &io) {
					error!("title capture: {err:#}");
					exit.write(AppExit::from_code(1));
					return;
				}
				exit.write(AppExit::Success);
				return;
			}
			state.stage = Stage::Shot { frame: frame + 1, requested: false };
		}
		Stage::Encode => {}
	}
}

fn finish_capture(config: &TitleSequenceConfig, io: &CaptureIo) -> anyhow::Result<()> {
	std::fs::create_dir_all(&io.stills_dir)?;
	encode_png_sequence(&io.ffmpeg, config.fps, &io.frames_dir, &io.output)?;
	info!("title mp4: {}", io.output.display());

	apply_swallow_alpha(&config.sequence, config.fps, &io.frames_dir)?;
	for key in Keyframe::ALL {
		let frame = config.sequence.frame_at(key.time, config.fps);
		let src = io.frames_dir.join(format!("frame_{frame:05}.png"));
		let dest = io.stills_dir.join(format!("{}.png", key.name));
		if key == Keyframe::SWALLOW_MID || key == Keyframe::SWALLOW_CLEAR {
			composite_over_checkerboard(&src, &dest)?;
		} else {
			std::fs::copy(&src, &dest).map_err(|err| {
				anyhow::anyhow!("copy {} → {}: {err}", src.display(), dest.display())
			})?;
		}
		info!("title still: {}", dest.display());
	}

	encode_overlay_sequence(&io.ffmpeg, config.fps, &io.frames_dir, &io.overlay)?;
	info!("title overlay: {}", io.overlay.display());
	if !io.keep_frames {
		let _ = std::fs::remove_dir_all(&io.frames_dir);
	}
	Ok(())
}
