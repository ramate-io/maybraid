//! Temporary flipbook atlases and shared GPU handles.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Texture handle, atlas grid, frame count, and playback duration.
#[derive(Clone, Debug)]
pub struct FlipbookAsset {
	pub image: Handle<Image>,
	pub grid: UVec2,
	pub frame_count: u32,
	pub playback: f32,
}

impl FlipbookAsset {
	pub fn last_frame(&self) -> u32 {
		self.frame_count.saturating_sub(1)
	}
}

pub const FIRE_GRID: UVec2 = UVec2::new(4, 4);
pub const SMOKE_GRID: UVec2 = UVec2::new(4, 4);
pub const ATLAS_CELL: u32 = 64;

/// Warm expanding burst. 16 frames over [`FIRE_PLAYBACK`] seconds.
pub const FIRE_PLAYBACK: f32 = 0.45;
/// Soft rising puff. 16 frames over [`SMOKE_PLAYBACK`] seconds.
pub const SMOKE_PLAYBACK: f32 = 1.6;

pub fn fire_flipbook(images: &mut Assets<Image>) -> FlipbookAsset {
	FlipbookAsset {
		image: images.add(paint_atlas(FIRE_GRID, fire_cell)),
		grid: FIRE_GRID,
		frame_count: FIRE_GRID.x * FIRE_GRID.y,
		playback: FIRE_PLAYBACK,
	}
}

pub fn smoke_flipbook(images: &mut Assets<Image>) -> FlipbookAsset {
	FlipbookAsset {
		image: images.add(paint_atlas(SMOKE_GRID, smoke_cell)),
		grid: SMOKE_GRID,
		frame_count: SMOKE_GRID.x * SMOKE_GRID.y,
		playback: SMOKE_PLAYBACK,
	}
}

fn paint_atlas(grid: UVec2, cell: fn(f32, f32, f32) -> [u8; 4]) -> Image {
	let cell_px = ATLAS_CELL;
	let width = grid.x * cell_px;
	let height = grid.y * cell_px;
	let frames = grid.x * grid.y;
	let mut data = vec![0u8; (width * height * 4) as usize];
	for frame in 0..frames {
		let t = if frames > 1 { frame as f32 / (frames - 1) as f32 } else { 0.0 };
		let col = frame % grid.x;
		let row = frame / grid.x;
		for y in 0..cell_px {
			for x in 0..cell_px {
				let u = (x as f32 + 0.5) / cell_px as f32;
				let v = (y as f32 + 0.5) / cell_px as f32;
				let px = col * cell_px + x;
				let py = row * cell_px + y;
				let i = ((py * width + px) * 4) as usize;
				data[i..i + 4].copy_from_slice(&cell(u, v, t));
			}
		}
	}
	let mut image = Image::new(
		Extent3d { width, height, depth_or_array_layers: 1 },
		TextureDimension::D2,
		data,
		TextureFormat::Rgba8UnormSrgb,
		RenderAssetUsages::RENDER_WORLD,
	);
	image.sampler = ImageSampler::linear();
	image
}

fn fire_cell(u: f32, v: f32, t: f32) -> [u8; 4] {
	let dx = u - 0.5;
	let dy = v - 0.5;
	let r = (dx * dx + dy * dy).sqrt();
	let radius = 0.14 + 0.34 * t;
	let core = (1.0 - (r / radius.max(1e-3)).clamp(0.0, 1.0)).powf(1.35);
	let lobe = (core * (1.1 - 0.35 * t)).clamp(0.0, 1.0);
	let hot = Vec3::new(1.0, 0.92, 0.55);
	let mid = Vec3::new(1.0, 0.38, 0.06);
	let cool = Vec3::new(0.28, 0.04, 0.01);
	let color = if t < 0.45 { hot.lerp(mid, t / 0.45) } else { mid.lerp(cool, (t - 0.45) / 0.55) };
	let alpha = lobe * (1.0 - t * 0.72);
	rgba(color, alpha)
}

fn smoke_cell(u: f32, v: f32, t: f32) -> [u8; 4] {
	let dx = u - 0.5;
	let dy = v - 0.52;
	let r = (dx * dx * 0.85 + dy * dy * 1.15).sqrt();
	let radius = 0.18 + 0.32 * t;
	let soft = (1.0 - (r / radius.max(1e-3)).clamp(0.0, 1.0)).powf(1.8);
	let gray = 0.22 + 0.16 * (1.0 - t);
	let color = Vec3::splat(gray);
	let alpha = soft * (0.55 * (1.0 - t).powf(1.1));
	rgba(color, alpha)
}

fn rgba(color: Vec3, alpha: f32) -> [u8; 4] {
	[
		(color.x.clamp(0.0, 1.0) * 255.0) as u8,
		(color.y.clamp(0.0, 1.0) * 255.0) as u8,
		(color.z.clamp(0.0, 1.0) * 255.0) as u8,
		(alpha.clamp(0.0, 1.0) * 255.0) as u8,
	]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fire_first_frame_is_a_hot_disk() {
		let px = fire_cell(0.5, 0.5, 0.0);
		assert!(px[0] > 200 && px[3] > 160, "{px:?}");
		let edge = fire_cell(0.02, 0.02, 0.0);
		assert_eq!(edge[3], 0);
	}

	#[test]
	fn smoke_last_frame_is_thin() {
		let center = smoke_cell(0.5, 0.5, 1.0);
		assert!(center[3] < 40, "{center:?}");
	}
}
