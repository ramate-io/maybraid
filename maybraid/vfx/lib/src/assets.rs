//! Authored-style flipbook atlases and the spark taper mask.

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

pub const FIRE_GRID: UVec2 = UVec2::new(8, 8);
pub const SMOKE_GRID: UVec2 = UVec2::new(8, 8);
pub const ATLAS_CELL: u32 = 64;

/// Fire sequence length. Sprite index is mapped to particle lifetime.
pub const FIRE_PLAYBACK: f32 = 0.55;
/// Smoke sequence length. Last frames stay a visible puff.
pub const SMOKE_PLAYBACK: f32 = 2.4;

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

/// Horizontal streak with a tapered alpha mask.
pub fn spark_taper(images: &mut Assets<Image>) -> Handle<Image> {
	images.add(paint_spark_taper())
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
	atlas_image(width, height, data)
}

fn paint_spark_taper() -> Image {
	const W: u32 = 64;
	const H: u32 = 16;
	let mut data = vec![0u8; (W * H * 4) as usize];
	for y in 0..H {
		for x in 0..W {
			let u = (x as f32 + 0.5) / W as f32;
			let v = (y as f32 + 0.5) / H as f32;
			let along = 1.0 - ((u - 0.5).abs() * 2.0).powf(1.6);
			let across = 1.0 - ((v - 0.5).abs() * 2.2).clamp(0.0, 1.0);
			let alpha = (along * across.powf(1.4)).clamp(0.0, 1.0);
			let hot = Vec3::new(1.0, 0.86, 0.42);
			let i = ((y * W + x) * 4) as usize;
			data[i..i + 4].copy_from_slice(&rgba(hot, alpha));
		}
	}
	atlas_image(W, H, data)
}

fn atlas_image(width: u32, height: u32, data: Vec<u8>) -> Image {
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
	let swell = 0.72 + 0.38 * (t * std::f32::consts::PI).sin().max(0.0);
	let mut cover: f32 = 0.0;
	let mut heat: f32 = 0.0;
	for (i, &(ox, oy, w)) in
		[(0.00, 0.02, 1.00), (0.16, -0.06, 0.72), (-0.14, 0.10, 0.68), (0.04, 0.16, 0.55)]
			.iter()
			.enumerate()
	{
		let drift = (t * (1.2 + i as f32 * 0.35) + i as f32).sin() * 0.06;
		let cx = 0.5 + ox * swell + drift;
		let cy = 0.5 + oy * swell - t * 0.04;
		let r = ((u - cx) * (u - cx) + (v - cy) * (v - cy) * 0.92).sqrt();
		let radius = (0.16 + 0.10 * w) * swell * (1.0 - t * 0.22);
		let blob = (1.0 - (r / radius.max(1e-3)).clamp(0.0, 1.0)).powf(1.45);
		cover = cover.max(blob);
		heat = heat.max(blob * (1.05 - r * 1.8));
	}
	let margin = edge_margin(u, v);
	cover *= margin;
	heat *= margin;
	let hot = Vec3::new(1.0, 0.90, 0.42);
	let mid = Vec3::new(0.98, 0.36, 0.07);
	let cool = Vec3::new(0.30, 0.05, 0.02);
	let shade = soft_band((heat * (1.0 - t * 0.35)).clamp(0.0, 1.0), 4.0);
	let color = if shade > 0.55 {
		hot.lerp(mid, (1.0 - shade) / 0.45)
	} else {
		mid.lerp(cool, 1.0 - shade / 0.55)
	};
	let alpha = cover * (0.95 - t * 0.25);
	rgba(color, alpha)
}

fn smoke_cell(u: f32, v: f32, t: f32) -> [u8; 4] {
	let swell = 0.78 + 0.42 * t;
	let mut cover: f32 = 0.0;
	let mut lit: f32 = 0.0;
	for (i, &(ox, oy, w)) in [
		(0.00, 0.04, 1.00),
		(0.14, -0.08, 0.78),
		(-0.16, 0.02, 0.74),
		(0.08, 0.14, 0.62),
		(-0.06, -0.12, 0.58),
	]
	.iter()
	.enumerate()
	{
		let drift = (t * (0.9 + i as f32 * 0.4) + i as f32 * 1.7).sin() * 0.07;
		let cx = 0.5 + ox * swell + drift;
		let cy = 0.52 + oy * swell - t * 0.10;
		let r = ((u - cx) * (u - cx) * 0.88 + (v - cy) * (v - cy) * 1.05).sqrt();
		let radius = (0.18 + 0.11 * w) * swell;
		let blob = (1.0 - (r / radius.max(1e-3)).clamp(0.0, 1.0)).powf(1.7);
		cover = cover.max(blob);
		lit = lit.max(blob * (0.55 + 0.45 * (1.0 - (v - cy).abs() * 2.2).clamp(0.0, 1.0)));
	}
	let margin = edge_margin(u, v);
	cover *= margin;
	let shade = soft_band(lit.clamp(0.0, 1.0), 3.0);
	let warm = Vec3::new(0.48, 0.38, 0.30);
	let cool = Vec3::new(0.20, 0.23, 0.28);
	let color = cool.lerp(warm, shade);
	let alpha = cover * (0.62 - t * 0.18).max(0.28);
	rgba(color, alpha)
}

fn edge_margin(u: f32, v: f32) -> f32 {
	let mx = (u.min(1.0 - u) * 8.0).clamp(0.0, 1.0);
	let my = (v.min(1.0 - v) * 8.0).clamp(0.0, 1.0);
	mx.min(my)
}

fn soft_band(v: f32, steps: f32) -> f32 {
	let s = steps.max(2.0);
	let x = v.clamp(0.0, 1.0) * s;
	let q = x.floor();
	let f = x.fract();
	let t = ((f - 0.32) / 0.36).clamp(0.0, 1.0);
	let t = t * t * (3.0 - 2.0 * t);
	(q + t) / s
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
	fn fire_first_frame_has_a_hot_core() {
		let px = fire_cell(0.5, 0.5, 0.0);
		assert!(px[0] > 180 && px[3] > 80, "{px:?}");
		let edge = fire_cell(0.02, 0.02, 0.0);
		assert_eq!(edge[3], 0);
	}

	#[test]
	fn smoke_last_frame_holds_a_puff() {
		let center = smoke_cell(0.5, 0.48, 1.0);
		assert!(center[3] > 40, "{center:?}");
		let edge = smoke_cell(0.02, 0.02, 1.0);
		assert_eq!(edge[3], 0);
	}

	#[test]
	fn spark_mask_tapers() {
		let image = paint_spark_taper();
		let data = image.data.as_ref().expect("spark bytes");
		let mid = ((8u32 * 64 + 32) * 4) as usize;
		let tip = ((8u32 * 64 + 2) * 4) as usize;
		assert!(data[mid + 3] > 160);
		assert!(data[tip + 3] < 40);
	}
}
