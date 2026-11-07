//! PNG sequence → MP4 via ffmpeg.

use std::path::{Path, PathBuf};
use std::process::Command;

/// ffmpeg argv after the binary name.
pub fn ffmpeg_args(fps: u32, frames_dir: &Path, output: &Path) -> Vec<String> {
	let pattern = frames_dir.join("frame_%05d.png");
	vec![
		"-y".into(),
		"-loglevel".into(),
		"error".into(),
		"-framerate".into(),
		fps.to_string(),
		"-i".into(),
		pattern.display().to_string(),
		"-c:v".into(),
		"libx264".into(),
		"-pix_fmt".into(),
		"yuv420p".into(),
		"-crf".into(),
		"18".into(),
		"-movflags".into(),
		"+faststart".into(),
		output.display().to_string(),
	]
}

/// Encode `frames_dir/frame_XXXXX.png` into `output`.
pub fn encode_png_sequence(
	ffmpeg: &Path,
	fps: u32,
	frames_dir: &Path,
	output: &Path,
) -> anyhow::Result<PathBuf> {
	if let Some(parent) = output.parent() {
		if !parent.as_os_str().is_empty() {
			std::fs::create_dir_all(parent)?;
		}
	}
	let args = ffmpeg_args(fps, frames_dir, output);
	let status = Command::new(ffmpeg)
		.args(&args)
		.status()
		.map_err(|err| anyhow::anyhow!("failed to spawn {}: {err}", ffmpeg.display()))?;
	if !status.success() {
		return Err(anyhow::anyhow!("ffmpeg exited with {status} ({})", ffmpeg.display()));
	}
	if !output.is_file() {
		return Err(anyhow::anyhow!("ffmpeg did not write {}", output.display()));
	}
	Ok(output.to_path_buf())
}

#[cfg(test)]
mod tests {
	use super::ffmpeg_args;
	use std::path::Path;

	#[test]
	fn ffmpeg_args_are_frame_exact_h264() -> anyhow::Result<()> {
		let args = ffmpeg_args(60, Path::new("/tmp/frames"), Path::new("/tmp/out.mp4"));
		assert_eq!(args[0], "-y");
		let fps = args
			.windows(2)
			.find(|pair| pair[0] == "-framerate")
			.ok_or_else(|| anyhow::anyhow!("missing -framerate"))?;
		assert_eq!(fps[1], "60");
		assert!(args.iter().any(|arg| arg == "libx264"));
		assert!(args.iter().any(|arg| arg == "yuv420p"));
		assert!(args.iter().any(|arg| arg.contains("frame_%05d.png")));
		assert_eq!(args.last().map(String::as_str), Some("/tmp/out.mp4"));
		Ok(())
	}
}
