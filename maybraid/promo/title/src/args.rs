//! CLI / env configuration for the title-sequence binary.

use std::path::PathBuf;

use clap::Parser;

use crate::timeline::{DEFAULT_FPS, DEFAULT_HEIGHT, DEFAULT_WIDTH, DURATION_SECS};

/// Render a deterministic ~5 s Maybraid title sequence.
#[derive(Parser, Debug, Clone)]
#[command(
	name = "promo-title",
	about = "Scripted Maybraid title sequence: rockadder/cosimo background, fallen-star icon, pulsing menu label, optional MP4 capture."
)]
pub struct Args {
	/// Menu label drawn with the existing header face after the star lands.
	#[arg(long, env = "PROMO_TITLE_LABEL", default_value = "Maybraid")]
	pub label: String,

	/// Output width in pixels.
	#[arg(long, env = "PROMO_TITLE_WIDTH", default_value_t = DEFAULT_WIDTH)]
	pub width: u32,

	/// Output height in pixels.
	#[arg(long, env = "PROMO_TITLE_HEIGHT", default_value_t = DEFAULT_HEIGHT)]
	pub height: u32,

	/// Capture / timeline frames per second.
	#[arg(long, env = "PROMO_TITLE_FPS", default_value_t = DEFAULT_FPS)]
	pub fps: u32,

	/// Sequence length in seconds. Authored motion is 5 s; override only for tests.
	#[arg(long, env = "PROMO_TITLE_DURATION", default_value_t = DURATION_SECS)]
	pub duration: f32,

	/// Write an MP4 here (PNG sequence + ffmpeg). Omit to play the sequence live.
	#[arg(long, env = "PROMO_TITLE_OUTPUT")]
	pub output: Option<PathBuf>,

	/// ffmpeg binary. Defaults to `ffmpeg` on PATH.
	#[arg(long, env = "PROMO_TITLE_FFMPEG", default_value = "ffmpeg")]
	pub ffmpeg: PathBuf,

	/// Keep the intermediate `frame_XXXXX.png` directory next to the MP4.
	#[arg(long, env = "PROMO_TITLE_KEEP_FRAMES", default_value_t = false)]
	pub keep_frames: bool,

	/// Extra GPU frames before frame 0, so shaders and fonts settle.
	#[arg(long, env = "PROMO_TITLE_WARMUP_FRAMES", default_value_t = 48)]
	pub warmup_frames: u32,
}

#[cfg(test)]
mod tests {
	use clap::Parser;

	use super::Args;

	#[test]
	fn parses_custom_label_resolution_and_fps() -> anyhow::Result<()> {
		let args = Args::try_parse_from([
			"promo-title",
			"--label",
			"Coming Soon",
			"--width",
			"1280",
			"--height",
			"720",
			"--fps",
			"30",
			"--output",
			"out.mp4",
		])
		.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert_eq!(args.label, "Coming Soon");
		assert_eq!(args.width, 1280);
		assert_eq!(args.height, 720);
		assert_eq!(args.fps, 30);
		assert_eq!(args.output.as_deref(), Some(std::path::Path::new("out.mp4")));
		Ok(())
	}

	#[test]
	fn defaults_match_the_documented_command() -> anyhow::Result<()> {
		let args = Args::try_parse_from(["promo-title"]).map_err(|err| anyhow::anyhow!("{err}"))?;
		assert_eq!(args.label, "Maybraid");
		assert_eq!(args.width, 1920);
		assert_eq!(args.height, 1080);
		assert_eq!(args.fps, 60);
		assert!(args.output.is_none());
		Ok(())
	}
}
