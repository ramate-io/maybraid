//! PNG sequence → H.264 preview and a ProRes 4444 overlay with swallow alpha.

use std::path::{Path, PathBuf};
use std::process::Command;

use image::{RgbImage, RgbaImage};

use crate::timeline::{Swallow, TitleSequence};

const CHECKER_CELL: u32 = 32;

/// ffmpeg argv after the binary name for the opaque H.264 preview.
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

/// ffmpeg argv for the durop overlay: ProRes 4444 MOV with a real alpha plane.
pub fn ffmpeg_overlay_args(fps: u32, frames_dir: &Path, output: &Path) -> Vec<String> {
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
		"prores_ks".into(),
		"-profile:v".into(),
		"4444".into(),
		"-pix_fmt".into(),
		"yuva444p10le".into(),
		"-qscale:v".into(),
		"13".into(),
		"-vendor".into(),
		"apl0".into(),
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
	run_ffmpeg(ffmpeg, &ffmpeg_args(fps, frames_dir, output), output)
}

/// Encode the RGBA swallow sequence as ProRes 4444 MOV.
pub fn encode_overlay_sequence(
	ffmpeg: &Path,
	fps: u32,
	frames_dir: &Path,
	output: &Path,
) -> anyhow::Result<PathBuf> {
	run_ffmpeg(ffmpeg, &ffmpeg_overlay_args(fps, frames_dir, output), output)
}

/// ffmpeg argv: RGBA frames over a looping checkerboard, H.264 proof video.
pub fn ffmpeg_checkerboard_args(
	fps: u32,
	frames_dir: &Path,
	checker: &Path,
	output: &Path,
) -> Vec<String> {
	let pattern = frames_dir.join("frame_%05d.png");
	vec![
		"-y".into(),
		"-loglevel".into(),
		"error".into(),
		"-loop".into(),
		"1".into(),
		"-framerate".into(),
		fps.to_string(),
		"-i".into(),
		checker.display().to_string(),
		"-framerate".into(),
		fps.to_string(),
		"-i".into(),
		pattern.display().to_string(),
		"-filter_complex".into(),
		"[1:v]format=rgba[fg];[0:v][fg]overlay=shortest=1".into(),
		"-c:v".into(),
		"libx264".into(),
		"-pix_fmt".into(),
		"yuv420p".into(),
		"-crf".into(),
		"18".into(),
		"-r".into(),
		fps.to_string(),
		"-movflags".into(),
		"+faststart".into(),
		output.display().to_string(),
	]
}

/// Composite the punched RGBA sequence over a checkerboard and encode H.264.
pub fn encode_checkerboard_preview(
	ffmpeg: &Path,
	fps: u32,
	frames_dir: &Path,
	output: &Path,
) -> anyhow::Result<PathBuf> {
	let first = frames_dir.join("frame_00000.png");
	let image = image::open(&first)
		.map_err(|err| anyhow::anyhow!("{}: {err}", first.display()))?
		.to_rgba8();
	let (width, height) = image.dimensions();
	let checker = frames_dir.join(".checker.png");
	write_checker_png(&checker, width, height)?;
	run_ffmpeg(ffmpeg, &ffmpeg_checkerboard_args(fps, frames_dir, &checker, output), output)
}

fn write_checker_png(path: &Path, width: u32, height: u32) -> anyhow::Result<()> {
	let mut image = RgbImage::new(width.max(1), height.max(1));
	for (x, y, pixel) in image.enumerate_pixels_mut() {
		*pixel = image::Rgb(checker_rgb(x, y));
	}
	image
		.save(path)
		.map_err(|err| anyhow::anyhow!("write {}: {err}", path.display()))?;
	Ok(())
}

fn run_ffmpeg(ffmpeg: &Path, args: &[String], output: &Path) -> anyhow::Result<PathBuf> {
	if let Some(parent) = output.parent() {
		if !parent.as_os_str().is_empty() {
			std::fs::create_dir_all(parent)?;
		}
	}
	let status = Command::new(ffmpeg)
		.args(args)
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

/// Punch the circular swallow into every `frame_XXXXX.png` as straight RGBA.
pub fn apply_swallow_alpha(
	sequence: &TitleSequence,
	fps: u32,
	frames_dir: &Path,
) -> anyhow::Result<()> {
	let total = sequence.frame_count(fps);
	for frame in 0..total {
		let path = frames_dir.join(format!("frame_{frame:05}.png"));
		let time = sequence.time_of_frame(frame, fps);
		let swallow = sequence.sample(time).swallow;
		punch_swallow_file(&path, swallow)?;
	}
	Ok(())
}

fn punch_swallow_file(path: &Path, swallow: Swallow) -> anyhow::Result<()> {
	let mut image = image::open(path)
		.map_err(|err| anyhow::anyhow!("{}: {err}", path.display()))?
		.to_rgba8();
	apply_swallow_to_rgba(&mut image, swallow);
	image
		.save(path)
		.map_err(|err| anyhow::anyhow!("write {}: {err}", path.display()))?;
	Ok(())
}

/// Multiply each pixel's alpha by the swallow coverage.
pub fn apply_swallow_to_rgba(image: &mut RgbaImage, swallow: Swallow) {
	if swallow.radius <= 0.0 {
		return;
	}
	let (width, height) = image.dimensions();
	for (x, y, pixel) in image.enumerate_pixels_mut() {
		let coverage = swallow.scene_alpha(x, y, width, height);
		pixel.0[3] = (pixel.0[3] as f32 * coverage).round() as u8;
	}
}

/// Composite an RGBA still over a checkerboard so transparency is visible in RGB.
pub fn composite_over_checkerboard(src: &Path, dest: &Path) -> anyhow::Result<()> {
	let image = image::open(src)
		.map_err(|err| anyhow::anyhow!("{}: {err}", src.display()))?
		.to_rgba8();
	let preview = checkerboard_preview(&image);
	if let Some(parent) = dest.parent() {
		if !parent.as_os_str().is_empty() {
			std::fs::create_dir_all(parent)?;
		}
	}
	preview
		.save(dest)
		.map_err(|err| anyhow::anyhow!("write {}: {err}", dest.display()))?;
	Ok(())
}

fn checkerboard_preview(image: &RgbaImage) -> RgbImage {
	let (width, height) = image.dimensions();
	let mut out = RgbImage::new(width, height);
	for (x, y, pixel) in image.enumerate_pixels() {
		let alpha = pixel.0[3] as f32 / 255.0;
		let [bg_r, bg_g, bg_b] = checker_rgb(x, y);
		out.put_pixel(
			x,
			y,
			image::Rgb([
				lerp_u8(bg_r, pixel.0[0], alpha),
				lerp_u8(bg_g, pixel.0[1], alpha),
				lerp_u8(bg_b, pixel.0[2], alpha),
			]),
		);
	}
	out
}

fn checker_rgb(x: u32, y: u32) -> [u8; 3] {
	if ((x / CHECKER_CELL) + (y / CHECKER_CELL)).is_multiple_of(2) {
		[214, 214, 218]
	} else {
		[46, 48, 56]
	}
}

fn lerp_u8(bg: u8, fg: u8, alpha: f32) -> u8 {
	(bg as f32 * (1.0 - alpha) + fg as f32 * alpha).round() as u8
}

#[cfg(test)]
mod tests {
	use super::{
		apply_swallow_to_rgba, checkerboard_preview, ffmpeg_args, ffmpeg_checkerboard_args,
		ffmpeg_overlay_args,
	};
	use crate::timeline::{Keyframe, TitleSequence};
	use image::{Rgba, RgbaImage};
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

	#[test]
	fn overlay_args_are_prores_4444_with_alpha() -> anyhow::Result<()> {
		let args = ffmpeg_overlay_args(60, Path::new("/tmp/frames"), Path::new("/tmp/out.mov"));
		assert!(args.iter().any(|arg| arg == "prores_ks"));
		assert!(args.iter().any(|arg| arg == "4444"));
		assert!(args.iter().any(|arg| arg == "yuva444p10le"));
		let qscale = args
			.windows(2)
			.find(|pair| pair[0] == "-qscale:v")
			.ok_or_else(|| anyhow::anyhow!("missing -qscale:v"))?;
		assert_eq!(qscale[1], "13");
		assert_eq!(args.last().map(String::as_str), Some("/tmp/out.mov"));
		Ok(())
	}

	#[test]
	fn checkerboard_args_overlay_rgba_on_a_still() -> anyhow::Result<()> {
		let args = ffmpeg_checkerboard_args(
			60,
			Path::new("/tmp/frames"),
			Path::new("/tmp/checker.png"),
			Path::new("/tmp/out-checker.mp4"),
		);
		assert!(args.iter().any(|arg| arg.contains("overlay=shortest=1")));
		assert!(args.iter().any(|arg| arg == "libx264"));
		assert!(args.iter().any(|arg| arg.contains("frame_%05d.png")));
		Ok(())
	}

	#[test]
	fn swallow_punch_clears_the_label_center() -> anyhow::Result<()> {
		let swallow = TitleSequence::new("Maybraid").sample(Keyframe::SWALLOW_MID.time).swallow;
		let mut image = RgbaImage::from_pixel(64, 36, Rgba([255, 0, 0, 255]));
		apply_swallow_to_rgba(&mut image, swallow);
		assert_eq!(image.get_pixel(32, 18).0[3], 0);
		assert_eq!(image.get_pixel(1, 1).0[3], 255);
		Ok(())
	}

	#[test]
	fn checkerboard_preview_shows_the_background_through_a_hole() -> anyhow::Result<()> {
		let mut image = RgbaImage::from_pixel(64, 64, Rgba([255, 0, 0, 255]));
		image.put_pixel(0, 0, Rgba([0, 0, 0, 0]));
		let preview = checkerboard_preview(&image);
		assert_eq!(preview.get_pixel(0, 0).0, [214, 214, 218]);
		assert_eq!(preview.get_pixel(32, 0).0, [255, 0, 0]);
		Ok(())
	}
}
