//! Tileable crack-distance atlas for the mythical-chain material.
//!
//! Four 128×128 variants are packed in a 2×2, 256×256 RGBA atlas:
//!
//! - **R** — distance to the nearest fissure (`0` on the crack, `1` far)
//! - **G** — recessed height (`0` deep center, `~0.5` intact metal)
//! - **B** — cell id for fracture-driven piece removal
//! - **A** — `255`
//!
//! The bake is deterministic (same [`crate::hash::hash13`] as the shader).
//! [`write_placeholder_png`] writes the committed artist placeholder;
//! runtime sampling uses [`atlas_image`] so GPU and PNG stay in lockstep.

use std::f32::consts::TAU;
use std::path::Path;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use crate::hash::hash13;

/// Shared GPU handle for the baked atlas. Created once per app.
#[derive(Resource, Default, Debug)]
pub struct ChainCrackAtlas {
	pub handle: Option<Handle<Image>>,
}

impl ChainCrackAtlas {
	pub fn get_or_create(&mut self, images: &mut Assets<Image>) -> Handle<Image> {
		self.handle.get_or_insert_with(|| images.add(atlas_image())).clone()
	}
}

/// One variant edge in pixels.
pub const VARIANT_SIZE: u32 = 128;
/// Atlas edge in pixels (2×2 variants).
pub const ATLAS_SIZE: u32 = VARIANT_SIZE * 2;
/// Number of packed crack networks.
pub const VARIANT_COUNT: u32 = 4;
/// Ring/tube aspect used when sampling in link-local UV.
pub const CRACK_ASPECT: f32 = 3.05;
/// Distance (in tile units) that maps to atlas red = 1.
const DIST_RANGE: f32 = 0.16;

#[derive(Clone, Copy, Debug)]
struct Segment {
	a: Vec2,
	b: Vec2,
}

/// Cached atlas bytes. The bake is deterministic and not cheap.
pub fn baked_atlas_rgba() -> &'static [u8] {
	use std::sync::OnceLock;
	static ATLAS: OnceLock<Vec<u8>> = OnceLock::new();
	ATLAS.get_or_init(bake_atlas_rgba)
}

/// Bake the four-variant atlas as tightly packed RGBA8.
pub fn bake_atlas_rgba() -> Vec<u8> {
	let mut pixels = vec![0u8; (ATLAS_SIZE * ATLAS_SIZE * 4) as usize];
	for variant in 0..VARIANT_COUNT {
		let field = bake_variant(variant);
		let ox = (variant % 2) * VARIANT_SIZE;
		let oy = (variant / 2) * VARIANT_SIZE;
		for y in 0..VARIANT_SIZE {
			for x in 0..VARIANT_SIZE {
				let dst = (((oy + y) * ATLAS_SIZE + (ox + x)) * 4) as usize;
				let src = ((y * VARIANT_SIZE + x) * 4) as usize;
				pixels[dst..dst + 4].copy_from_slice(&field[src..src + 4]);
			}
		}
	}
	pixels
}

/// GPU image of [`bake_atlas_rgba`], linear filtered, clamp-to-edge.
///
/// Wrapping is done in the shader on link-local UVs before the atlas remap
/// so adjacent variants do not bleed.
pub fn atlas_image() -> Image {
	let mut image = Image::new(
		Extent3d { width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1 },
		TextureDimension::D2,
		baked_atlas_rgba().to_vec(),
		TextureFormat::Rgba8Unorm,
		RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
	);
	image.sampler = ImageSampler::linear();
	image
}

/// Write the committed placeholder PNG (same bytes as [`bake_atlas_rgba`]).
pub fn write_placeholder_png(path: impl AsRef<Path>) -> std::io::Result<()> {
	let path = path.as_ref();
	if let Some(parent) = path.parent() {
		std::fs::create_dir_all(parent)?;
	}
	write_png_rgba(path, ATLAS_SIZE, ATLAS_SIZE, baked_atlas_rgba())
}

/// Sample the baked atlas on the CPU (tests / bake docs).
pub fn sample_atlas(rgba: &[u8], uv: Vec2, variant: u32) -> [u8; 4] {
	let variant = variant % VARIANT_COUNT;
	let local = Vec2::new(uv.x.rem_euclid(1.0), uv.y.rem_euclid(1.0));
	let ox = (variant % 2) * VARIANT_SIZE;
	let oy = (variant / 2) * VARIANT_SIZE;
	let x = (local.x * VARIANT_SIZE as f32).floor() as u32 % VARIANT_SIZE;
	let y = (local.y * VARIANT_SIZE as f32).floor() as u32 % VARIANT_SIZE;
	let i = (((oy + y) * ATLAS_SIZE + (ox + x)) * 4) as usize;
	[rgba[i], rgba[i + 1], rgba[i + 2], rgba[i + 3]]
}

fn bake_variant(variant: u32) -> Vec<u8> {
	let segments = grow_network(variant);
	let sites = cell_sites(variant);
	let n = VARIANT_SIZE as usize;
	let mut pixels = vec![0u8; n * n * 4];
	for y in 0..n {
		for x in 0..n {
			let u = (x as f32 + 0.5) / VARIANT_SIZE as f32;
			let v = (y as f32 + 0.5) / VARIANT_SIZE as f32;
			let p = Vec2::new(u, v);
			let dist = wrapped_min_dist(p, &segments);
			let norm = (dist / DIST_RANGE).clamp(0.0, 1.0);
			let recess = (1.0 - smoothstep(0.0, 0.028, dist)).clamp(0.0, 1.0);
			let height = 0.5 - recess * 0.48;
			let cell = nearest_site(p, &sites);
			let i = (y * n + x) * 4;
			pixels[i] = (norm * 255.0 + 0.5) as u8;
			pixels[i + 1] = (height * 255.0 + 0.5) as u8;
			pixels[i + 2] = (cell * 255.0 + 0.5) as u8;
			pixels[i + 3] = 255;
		}
	}
	pixels
}

fn grow_network(variant: u32) -> Vec<Segment> {
	let mut segments = Vec::new();
	// Two long primaries and a few secondary forks. Dense equal-weight
	// walks fill the tile so every fragment is "in a crack".
	let roots = 2u32;
	for root in 0..roots {
		let h0 = hash13([variant as f32 + 0.17, root as f32 * 1.9, 0.31]);
		let h1 = hash13([variant as f32 * 0.7, root as f32 + 2.4, 1.11]);
		let angle = hash13([h0, h1, 3.3]) * TAU;
		grow_branch(&mut segments, Vec2::new(h0, h1), angle, 0, variant, root, 14);
	}
	segments
}

fn grow_branch(
	segments: &mut Vec<Segment>,
	mut p: Vec2,
	mut angle: f32,
	depth: u32,
	variant: u32,
	root: u32,
	steps: u32,
) {
	let steps = if depth == 0 { steps } else { 6 };
	let mut forks = 0u32;
	for step in 0..steps {
		let salt = hash13([variant as f32, root as f32 * 5.1 + step as f32, depth as f32 + 0.4]);
		angle += (salt - 0.5) * if depth == 0 { 0.42 } else { 0.70 };
		let len = if depth == 0 {
			0.070 + 0.036 * hash13([salt, 8.2, depth as f32])
		} else {
			0.040 + 0.022 * hash13([salt, 8.2, depth as f32])
		};
		let next = p + Vec2::new(angle.cos(), angle.sin()) * len;
		push_wrapped_segment(segments, p, next);
		if depth == 0 && forks < 2 && salt < 0.22 && step > 3 && step < steps - 2 {
			let fork = angle + (if salt < 0.11 { 0.90 } else { -0.95 });
			grow_branch(segments, next, fork, 1, variant, root * 17 + step + 3, 6);
			forks += 1;
		}
		p = wrap01(next);
	}
}

fn push_wrapped_segment(segments: &mut Vec<Segment>, a: Vec2, b: Vec2) {
	let delta = b - a;
	if delta.x.abs() > 0.5 || delta.y.abs() > 0.5 {
		// Crossed a tile edge: split so distance tests stay local.
		let mid = wrap01(a + delta * 0.5);
		segments.push(Segment { a: wrap01(a), b: mid });
		segments.push(Segment { a: mid, b: wrap01(b) });
	} else {
		segments.push(Segment { a: wrap01(a), b: wrap01(b) });
	}
}

fn cell_sites(variant: u32) -> Vec<Vec2> {
	let mut sites = Vec::new();
	for y in 0..5 {
		for x in 0..5 {
			let jx = hash13([variant as f32, x as f32 + 0.2, y as f32 * 1.7]);
			let jy = hash13([y as f32 + 0.5, variant as f32 * 0.3, x as f32]);
			sites.push(Vec2::new(
				(x as f32 + 0.15 + jx * 0.7) / 5.0,
				(y as f32 + 0.15 + jy * 0.7) / 5.0,
			));
		}
	}
	sites
}

fn nearest_site(p: Vec2, sites: &[Vec2]) -> f32 {
	let mut best = f32::MAX;
	let mut idx = 0usize;
	for (i, site) in sites.iter().enumerate() {
		let d = wrapped_point_dist(p, *site);
		if d < best {
			best = d;
			idx = i;
		}
	}
	if sites.is_empty() {
		0.0
	} else {
		idx as f32 / (sites.len() as f32 - 1.0).max(1.0)
	}
}

fn wrapped_min_dist(p: Vec2, segments: &[Segment]) -> f32 {
	let mut best = f32::MAX;
	for seg in segments {
		best = best.min(wrapped_seg_dist(p, seg.a, seg.b));
	}
	best
}

fn wrapped_seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
	let mut best = f32::MAX;
	for oy in -1..=1 {
		for ox in -1..=1 {
			let off = Vec2::new(ox as f32, oy as f32);
			best = best.min(point_seg_dist(p, a + off, b + off));
		}
	}
	best
}

fn wrapped_point_dist(a: Vec2, b: Vec2) -> f32 {
	let mut d = a - b;
	d.x -= d.x.round();
	d.y -= d.y.round();
	d.length()
}

fn point_seg_dist(p: Vec2, a: Vec2, b: Vec2) -> f32 {
	let ab = b - a;
	let len2 = ab.length_squared();
	if len2 < 1e-10 {
		return (p - a).length();
	}
	let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
	(p - (a + ab * t)).length()
}

fn wrap01(p: Vec2) -> Vec2 {
	Vec2::new(p.x.rem_euclid(1.0), p.y.rem_euclid(1.0))
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	if (edge1 - edge0).abs() < 1e-6 {
		return if x < edge0 { 0.0 } else { 1.0 };
	}
	let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

/// Uncompressed PNG so the bake has no extra crate dependency.
fn write_png_rgba(path: &Path, width: u32, height: u32, rgba: &[u8]) -> std::io::Result<()> {
	use std::io::Write;
	let mut raw = Vec::with_capacity(((width * 4 + 1) * height) as usize);
	for y in 0..height as usize {
		raw.push(0);
		let row = y * width as usize * 4;
		raw.extend_from_slice(&rgba[row..row + width as usize * 4]);
	}
	let mut file = std::fs::File::create(path)?;
	file.write_all(&[137, 80, 78, 71, 13, 10, 26, 10])?;
	write_chunk(&mut file, b"IHDR", &{
		let mut ihdr = Vec::new();
		ihdr.extend_from_slice(&width.to_be_bytes());
		ihdr.extend_from_slice(&height.to_be_bytes());
		ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
		ihdr
	})?;
	write_chunk(&mut file, b"IDAT", &zlib_store(&raw))?;
	write_chunk(&mut file, b"IEND", &[])?;
	Ok(())
}

fn write_chunk(file: &mut std::fs::File, ty: &[u8; 4], data: &[u8]) -> std::io::Result<()> {
	use std::io::Write;
	file.write_all(&(data.len() as u32).to_be_bytes())?;
	file.write_all(ty)?;
	file.write_all(data)?;
	let mut crc_in = Vec::with_capacity(4 + data.len());
	crc_in.extend_from_slice(ty);
	crc_in.extend_from_slice(data);
	file.write_all(&crc32(&crc_in).to_be_bytes())?;
	Ok(())
}

fn zlib_store(data: &[u8]) -> Vec<u8> {
	let mut out = vec![0x78, 0x01];
	let mut offset = 0;
	while offset < data.len() {
		let end = (offset + 65535).min(data.len());
		let chunk = &data[offset..end];
		let last = end == data.len();
		out.push(if last { 1 } else { 0 });
		let n = chunk.len() as u16;
		out.extend_from_slice(&n.to_le_bytes());
		out.extend_from_slice(&(!n).to_le_bytes());
		out.extend_from_slice(chunk);
		offset = end;
	}
	out.extend_from_slice(&adler32(data).to_be_bytes());
	out
}

fn adler32(data: &[u8]) -> u32 {
	let mut a = 1u32;
	let mut b = 0u32;
	for &byte in data {
		a = (a + byte as u32) % 65521;
		b = (b + a) % 65521;
	}
	(b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
	let mut crc = 0xFFFF_FFFFu32;
	for &byte in data {
		crc ^= byte as u32;
		for _ in 0..8 {
			let mask = if crc & 1 == 1 { 0xEDB8_8320 } else { 0 };
			crc = (crc >> 1) ^ mask;
		}
	}
	!crc
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn atlas_is_tileable_and_branching() -> anyhow::Result<()> {
		let rgba = baked_atlas_rgba();
		assert_eq!(rgba.len(), (ATLAS_SIZE * ATLAS_SIZE * 4) as usize);
		for variant in 0..VARIANT_COUNT {
			let mut on_crack = 0u32;
			let mut min_d = 255u8;
			let mut max_d = 0u8;
			for i in 0..32 {
				let t = (i as f32 + 0.5) / 32.0;
				let left = sample_atlas(rgba, Vec2::new(0.0, t), variant)[0] as i32;
				let right =
					sample_atlas(rgba, Vec2::new(1.0 - 1.0 / VARIANT_SIZE as f32, t), variant)[0]
						as i32;
				assert!(
					(left - right).abs() < 40,
					"variant {variant} u-wrap drifted {left} vs {right} at v={t}"
				);
			}
			for y in (0..VARIANT_SIZE).step_by(8) {
				for x in (0..VARIANT_SIZE).step_by(8) {
					let d = sample_atlas(
						rgba,
						Vec2::new(
							(x as f32 + 0.5) / VARIANT_SIZE as f32,
							(y as f32 + 0.5) / VARIANT_SIZE as f32,
						),
						variant,
					)[0];
					if d < 24 {
						on_crack += 1;
					}
					min_d = min_d.min(d);
					max_d = max_d.max(d);
				}
			}
			assert!(on_crack > 2, "variant {variant} has no hairlines ({on_crack})");
			assert!(
				max_d > min_d.saturating_add(80),
				"variant {variant} should have solid metal away from cracks, min={min_d} max={max_d}"
			);
			assert!(
				max_d > 180,
				"variant {variant} needs open metal cells, max_d={max_d}"
			);
		}
		let a = sample_atlas(rgba, Vec2::new(0.2, 0.3), 0);
		let b = sample_atlas(rgba, Vec2::new(0.2, 0.3), 1);
		assert_ne!(a, b, "variants should differ at a shared UV");
		Ok(())
	}

	#[test]
	fn widening_threshold_reveals_more_of_the_same_network() -> anyhow::Result<()> {
		let rgba = baked_atlas_rgba();
		let mut hair = 0u32;
		let mut fissure = 0u32;
		for y in (0..VARIANT_SIZE).step_by(4) {
			for x in (0..VARIANT_SIZE).step_by(4) {
				let d = sample_atlas(
					rgba,
					Vec2::new(
						(x as f32 + 0.5) / VARIANT_SIZE as f32,
						(y as f32 + 0.5) / VARIANT_SIZE as f32,
					),
					0,
				)[0] as f32 / 255.0;
				if d < 0.08 {
					hair += 1;
				}
				if d < 0.28 {
					fissure += 1;
				}
			}
		}
		assert!(
			fissure > hair,
			"wider threshold should keep the same cracks: hair={hair} fissure={fissure}"
		);
		assert!(hair > 0);
		Ok(())
	}

	#[test]
	fn writes_placeholder_png() -> anyhow::Result<()> {
		let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/crack_atlas.png");
		write_placeholder_png(&path)?;
		let bytes = std::fs::read(&path)?;
		assert!(bytes.starts_with(&[137, 80, 78, 71]), "not a PNG");
		assert!(bytes.len() > 256);
		Ok(())
	}
}
