//! View-dependent grass parting: settings, occluders, and a cheap raster mask.

use bevy::math::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use bevy::prelude::*;
use vegetation_shaders::wesl::{
	clip_to_uv, world_part_from_screen, GrassPartingUniform, MAX_PARTING_OCCLUDERS,
	PARTING_MODE_DEPTH, PARTING_MODE_OFF, PARTING_MODE_RASTER, PARTING_MODE_WORLD,
};

/// How blades sample occluders. World is the production default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GrassPartingMode {
	Off,
	/// Sample the camera depth prepass at the projected tip (arbitrary opaque geometry).
	Depth,
	/// Low-res screen-space parting-vector footprint, CPU-splatted and blurred.
	Raster,
	/// World-space capsule field around tagged actors / props.
	#[default]
	World,
}

impl GrassPartingMode {
	pub fn as_u32(self) -> u32 {
		match self {
			Self::Off => PARTING_MODE_OFF,
			Self::Depth => PARTING_MODE_DEPTH,
			Self::Raster => PARTING_MODE_RASTER,
			Self::World => PARTING_MODE_WORLD,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Off => "off",
			Self::Depth => "depth",
			Self::Raster => "raster",
			Self::World => "world",
		}
	}
}

/// Strength, radius, and depth falloff. Authoring lives here, not in WESL.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrassPartingSettings {
	pub mode: GrassPartingMode,
	/// Tip deflection at full weight (metres).
	pub strength: f32,
	/// Extra influence around an occluder (metres). World mode only.
	pub radius: f32,
	/// Behind-the-blade window (metres) before the effect fades.
	pub depth_falloff: f32,
	/// Skip blades further than this from the camera.
	pub lod_skip_m: f32,
	/// Occluder world Y must exceed root Y by this much (rejects turf).
	pub ground_reject_m: f32,
	/// Depth-gradient kernel in pixels.
	pub kernel_px: f32,
	/// Raster stamp radius as a multiple of the occluder's real radius.
	pub stamp_scale: f32,
}

impl Default for GrassPartingSettings {
	fn default() -> Self {
		Self {
			mode: GrassPartingMode::World,
			strength: 0.78,
			radius: 1.25,
			depth_falloff: 2.4,
			lod_skip_m: 28.0,
			ground_reject_m: 0.18,
			kernel_px: 5.0,
			stamp_scale: 2.5,
		}
	}
}

impl GrassPartingSettings {
	pub fn off() -> Self {
		Self { mode: GrassPartingMode::Off, ..Self::default() }
	}

	pub fn with_mode(mut self, mode: GrassPartingMode) -> Self {
		self.mode = mode;
		self
	}

	pub fn with_stamp_scale(mut self, stamp_scale: f32) -> Self {
		self.stamp_scale = stamp_scale.max(0.05);
		self
	}

	pub fn enabled(self) -> bool {
		self.mode != GrassPartingMode::Off && self.strength > 1e-5
	}

	/// World-space stamp radius for raster occupancy (metres).
	pub fn stamp_radius_m(self, occluder_radius_m: f32) -> f32 {
		occluder_radius_m.max(0.05) * self.stamp_scale.max(0.05)
	}

	/// Build the GPU uniform from the game camera and extracted occluders.
	pub fn pack(
		self,
		clip_from_world: Mat4,
		camera: Vec3,
		camera_right: Vec3,
		camera_forward: Vec3,
		occluders: &[ExtractedPartingOccluder],
	) -> GrassPartingUniform {
		let mut uniform = GrassPartingUniform {
			camera_clip_from_world: clip_from_world,
			camera_world_from_clip: clip_from_world.inverse(),
			params: Vec4::new(
				self.strength.max(0.0),
				self.radius.max(0.0),
				self.depth_falloff.max(0.05),
				self.mode.as_u32() as f32,
			),
			extras: Vec4::new(
				self.lod_skip_m.max(1.0),
				self.ground_reject_m.max(0.0),
				self.kernel_px.max(1.0),
				occluders.len().min(MAX_PARTING_OCCLUDERS) as f32,
			),
			camera_right: camera_right.extend(0.0),
			camera_forward: camera_forward.extend(0.0),
			camera_position: camera.extend(0.0),
			..GrassPartingUniform::default()
		};
		for (i, occluder) in occluders.iter().take(MAX_PARTING_OCCLUDERS).enumerate() {
			uniform.occluders[i] = occluder.center.extend(occluder.radius_m);
			uniform.occluder_meta[i] = Vec4::new(occluder.height_m, 1.0, 0.0, 0.0);
		}
		uniform
	}
}

/// Tag an opaque actor / prop so world and raster modes can find it.
/// Depth mode also sees untagged StandardMaterial geometry via the prepass.
#[derive(Component, Clone, Copy, Debug)]
pub struct GrassPartingOccluder {
	pub radius_m: f32,
	pub height_m: f32,
}

impl GrassPartingOccluder {
	pub fn capsule(radius_m: f32, height_m: f32) -> Self {
		Self { radius_m, height_m }
	}

	pub fn character() -> Self {
		Self { radius_m: 0.4, height_m: 1.8 }
	}

	pub fn rock(radius_m: f32) -> Self {
		Self { radius_m, height_m: radius_m * 2.0 }
	}
}

/// CPU snapshot of one occluder, extracted each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExtractedPartingOccluder {
	pub center: Vec3,
	pub radius_m: f32,
	pub height_m: f32,
}

impl ExtractedPartingOccluder {
	pub fn from_transform(occluder: GrassPartingOccluder, transform: GlobalTransform) -> Self {
		Self {
			center: transform.translation(),
			radius_m: occluder.radius_m.max(0.05),
			height_m: occluder.height_m.max(0.1),
		}
	}
}

pub const PARTING_MASK_WIDTH: u32 = 256;
pub const PARTING_MASK_HEIGHT: u32 = 144;

/// Screen-space RG16 parting mask. RG = world XZ direction * weight, B = weight,
/// A = occluder distance from camera.
#[derive(Clone, Debug)]
pub struct PartingMask {
	pub width: u32,
	pub height: u32,
	pixels: Vec<[f32; 4]>,
}

impl PartingMask {
	pub fn new(width: u32, height: u32) -> Self {
		Self { width, height, pixels: vec![[0.0; 4]; (width * height) as usize] }
	}

	pub fn default_size() -> Self {
		Self::new(PARTING_MASK_WIDTH, PARTING_MASK_HEIGHT)
	}

	pub fn clear(&mut self) {
		self.pixels.fill([0.0; 4]);
	}

	pub fn pixel_bytes_rgba32(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(self.pixels.len() * 16);
		for pixel in &self.pixels {
			for channel in pixel {
				bytes.extend_from_slice(&channel.to_le_bytes());
			}
		}
		bytes
	}

	/// Splat projected occluder discs, then box-blur for temporal stability.
	pub fn splat(
		&mut self,
		occluders: &[ExtractedPartingOccluder],
		clip_from_world: Mat4,
		camera: Vec3,
		camera_right: Vec3,
		camera_forward: Vec3,
		settings: GrassPartingSettings,
	) {
		self.clear();
		for occluder in occluders.iter().take(MAX_PARTING_OCCLUDERS) {
			self.splat_one(
				occluder,
				clip_from_world,
				camera,
				camera_right,
				camera_forward,
				settings,
			);
		}
		self.blur();
	}

	fn splat_one(
		&mut self,
		occluder: &ExtractedPartingOccluder,
		clip_from_world: Mat4,
		camera: Vec3,
		camera_right: Vec3,
		camera_forward: Vec3,
		settings: GrassPartingSettings,
	) {
		let foot = Vec3::new(
			occluder.center.x,
			occluder.center.y - occluder.height_m * 0.5,
			occluder.center.z,
		);
		let clip = clip_from_world * foot.extend(1.0);
		if clip.w <= 1e-4 {
			return;
		}
		let uv = clip_to_uv(clip);
		if !(0.0..=1.0).contains(&uv.x) || !(0.0..=1.0).contains(&uv.y) {
			return;
		}
		let dist = foot.distance(camera);
		let radius_m = settings.stamp_radius_m(occluder.radius_m);
		let radius_px = projected_stamp_radius_px(
			foot,
			radius_m,
			clip_from_world,
			camera_right,
			self.width as f32,
			self.height as f32,
			dist,
		);
		let cx = uv.x * self.width as f32;
		let cy = uv.y * self.height as f32;
		let min_x = (cx - radius_px).floor().max(0.0) as u32;
		let max_x = (cx + radius_px).ceil().min(self.width as f32 - 1.0) as u32;
		let min_y = (cy - radius_px).floor().max(0.0) as u32;
		let max_y = (cy + radius_px).ceil().min(self.height as f32 - 1.0) as u32;
		let camera_up = camera_right.cross(camera_forward).normalize_or(Vec3::Y);
		for y in min_y..=max_y {
			for x in min_x..=max_x {
				let dx = x as f32 + 0.5 - cx;
				let dy = y as f32 + 0.5 - cy;
				let r = Vec2::new(dx, dy).length() / radius_px.max(1e-3);
				if r >= 1.0 {
					continue;
				}
				let weight = stamp_falloff(r);
				let mut dir_ss = Vec2::new(dx, dy);
				if dir_ss.length_squared() < 1e-4 {
					dir_ss = Vec2::X;
				} else {
					dir_ss = dir_ss.normalize();
				}
				let world_xz =
					world_part_from_screen(dir_ss, camera_right, camera_up, camera_forward).xz();
				let i = (y * self.width + x) as usize;
				self.pixels[i][0] += world_xz.x * weight;
				self.pixels[i][1] += world_xz.y * weight;
				self.pixels[i][2] += weight;
				if self.pixels[i][3] <= 1e-5 || dist < self.pixels[i][3] {
					self.pixels[i][3] = dist;
				}
			}
		}
	}

	#[cfg(test)]
	fn occupancy_count(&self, min_weight: f32) -> usize {
		self.pixels.iter().filter(|pixel| pixel[2] > min_weight).count()
	}

	fn blur(&mut self) {
		let src = self.pixels.clone();
		let w = self.width as i32;
		let h = self.height as i32;
		for y in 0..h {
			for x in 0..w {
				let mut acc = [0.0f32; 4];
				let mut n = 0.0;
				for oy in -1..=1 {
					for ox in -1..=1 {
						let sx = (x + ox).clamp(0, w - 1) as u32;
						let sy = (y + oy).clamp(0, h - 1) as u32;
						let p = src[(sy * self.width + sx) as usize];
						for c in 0..4 {
							acc[c] += p[c];
						}
						n += 1.0;
					}
				}
				self.pixels[(y as u32 * self.width + x as u32) as usize] =
					[acc[0] / n, acc[1] / n, acc[2] / n, acc[3] / n];
			}
		}
	}
}

/// Soft occupancy: full in the inner ~25% of the stamp, fading to the edge.
fn stamp_falloff(r: f32) -> f32 {
	1.0 - smoothstep(0.25, 1.0, r)
}

fn projected_stamp_radius_px(
	foot: Vec3,
	radius_m: f32,
	clip_from_world: Mat4,
	camera_right: Vec3,
	width: f32,
	height: f32,
	dist: f32,
) -> f32 {
	let mut right = camera_right;
	right.y = 0.0;
	let right = right.normalize_or(Vec3::X);
	let edge = foot + right * radius_m;
	let uv_c = clip_to_uv(clip_from_world * foot.extend(1.0));
	let uv_e = clip_to_uv(clip_from_world * edge.extend(1.0));
	let projected = ((uv_e - uv_c) * Vec2::new(width, height)).length();
	if projected.is_finite() && projected > 1.5 {
		projected.clamp(2.0, 160.0)
	} else {
		((radius_m / dist.max(0.2)) * height * 1.25).clamp(2.0, 160.0)
	}
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn raster_splat_points_away_from_projected_centre() -> anyhow::Result<()> {
		let mut mask = PartingMask::new(64, 36);
		let camera = Vec3::new(0.0, 0.6, 0.0);
		let clip = Mat4::perspective_rh(1.0, 16.0 / 9.0, 0.1, 80.0)
			* Mat4::look_at_rh(camera, Vec3::new(0.0, 0.6, 4.0), Vec3::Y);
		let occluder = ExtractedPartingOccluder {
			center: Vec3::new(0.0, 0.9, 4.0),
			radius_m: 0.4,
			height_m: 1.8,
		};
		mask.splat(
			&[occluder],
			clip,
			camera,
			Vec3::X,
			Vec3::Z,
			GrassPartingSettings::default().with_mode(GrassPartingMode::Raster),
		);
		let foot = Vec3::new(
			occluder.center.x,
			occluder.center.y - occluder.height_m * 0.5,
			occluder.center.z,
		);
		let mid = clip_to_uv(clip * foot.extend(1.0));
		let mx = (mid.x * 64.0) as u32;
		let my = (mid.y * 36.0) as u32;
		let left = mask.pixels[(my * 64 + mx.saturating_sub(4)) as usize];
		let right = mask.pixels[(my * 64 + (mx + 4).min(63)) as usize];
		assert!(left[2] > 0.0 || right[2] > 0.0, "expected a footprint around the capsule");
		assert!(
			left[0] <= right[0],
			"left pixel should peel more left than right, {left:?} vs {right:?}"
		);
		Ok(())
	}

	#[test]
	fn stamp_scale_multiplies_the_occluder_radius() -> anyhow::Result<()> {
		let settings = GrassPartingSettings::default().with_stamp_scale(2.5);
		assert!((settings.stamp_radius_m(0.4) - 1.0).abs() < 1e-5);
		let tight = GrassPartingSettings::default().with_stamp_scale(1.0);
		assert!((tight.stamp_radius_m(0.4) - 0.4).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn oversized_stamp_covers_beyond_the_capsule() -> anyhow::Result<()> {
		let camera = Vec3::new(0.0, 6.0, 0.0);
		let clip = Mat4::perspective_rh(1.0, 16.0 / 9.0, 0.1, 80.0)
			* Mat4::look_at_rh(camera, Vec3::new(0.0, 0.0, 0.05), Vec3::NEG_Z);
		let occluder = ExtractedPartingOccluder {
			center: Vec3::new(0.0, 0.9, 0.0),
			radius_m: 0.4,
			height_m: 1.8,
		};
		let splat = |scale: f32| {
			let mut mask = PartingMask::new(64, 36);
			mask.splat(
				&[occluder],
				clip,
				camera,
				Vec3::X,
				Vec3::NEG_Y,
				GrassPartingSettings::default()
					.with_mode(GrassPartingMode::Raster)
					.with_stamp_scale(scale),
			);
			mask
		};
		let tight = splat(1.0);
		let oversized = splat(3.0);
		assert!(
			oversized.occupancy_count(0.05) > tight.occupancy_count(0.05),
			"scale 3 should cover more texels than scale 1 ({} vs {})",
			oversized.occupancy_count(0.05),
			tight.occupancy_count(0.05)
		);
		Ok(())
	}

	#[test]
	fn stamp_falloff_is_full_in_the_core_and_soft_at_the_edge() -> anyhow::Result<()> {
		assert!((stamp_falloff(0.0) - 1.0).abs() < 1e-5);
		assert!(stamp_falloff(0.2) > 0.95);
		assert!(stamp_falloff(0.6) > 0.15 && stamp_falloff(0.6) < 0.85);
		assert!(stamp_falloff(1.0) < 1e-5);
		Ok(())
	}

	#[test]
	fn packing_counts_occluders() -> anyhow::Result<()> {
		let camera = Vec3::new(1.5, 0.6, -2.0);
		let uniform = GrassPartingSettings::default().pack(
			Mat4::IDENTITY,
			camera,
			Vec3::X,
			Vec3::Z,
			&[ExtractedPartingOccluder { center: Vec3::Y, radius_m: 0.4, height_m: 1.8 }],
		);
		assert_eq!(uniform.occluder_count(), 1);
		assert_eq!(uniform.mode(), PARTING_MODE_WORLD);
		assert_eq!(uniform.camera_world(), camera);
		Ok(())
	}
}
