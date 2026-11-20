//! View-dependent grass parting: settings, occluders, and a cheap raster mask.

use bevy::math::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use bevy::prelude::*;
use vegetation_shaders::wesl::{
	clip_to_uv, GrassPartingUniform, MAX_PARTING_OCCLUDERS, PARTING_MODE_DEPTH, PARTING_MODE_OFF,
	PARTING_MODE_RASTER, PARTING_MODE_WORLD,
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
	/// Extra influence around an occluder (metres).
	pub radius: f32,
	/// Behind-the-blade window (metres) before the effect fades.
	pub depth_falloff: f32,
	/// Skip blades further than this from the camera.
	pub lod_skip_m: f32,
	/// Occluder world Y must exceed root Y by this much (rejects turf).
	pub ground_reject_m: f32,
	/// Depth-gradient kernel in pixels.
	pub kernel_px: f32,
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

	pub fn enabled(self) -> bool {
		self.mode != GrassPartingMode::Off && self.strength > 1e-5
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
		let clip = clip_from_world * occluder.center.extend(1.0);
		if clip.w <= 1e-4 {
			return;
		}
		let uv = clip_to_uv(clip);
		if !(0.0..=1.0).contains(&uv.x) || !(0.0..=1.0).contains(&uv.y) {
			return;
		}
		let dist = occluder.center.distance(camera);
		let radius_m = occluder.radius_m + settings.radius;
		let radius_px = ((radius_m / dist.max(0.2)) * self.height as f32 * 1.25).clamp(2.0, 96.0);
		let aspect = (occluder.height_m / occluder.radius_m.max(0.05)).clamp(1.0, 4.0);
		let radius_py = (radius_px * aspect * 0.55).clamp(2.0, 120.0);
		let cx = uv.x * self.width as f32;
		let cy = uv.y * self.height as f32;
		let min_x = (cx - radius_px).floor().max(0.0) as u32;
		let max_x = (cx + radius_px).ceil().min(self.width as f32 - 1.0) as u32;
		let min_y = (cy - radius_py).floor().max(0.0) as u32;
		let max_y = (cy + radius_py).ceil().min(self.height as f32 - 1.0) as u32;
		let right_xz = {
			let mut r = camera_right;
			r.y = 0.0;
			r.normalize_or(Vec3::X).xz()
		};
		for y in min_y..=max_y {
			for x in min_x..=max_x {
				let dx = x as f32 + 0.5 - cx;
				let dy = y as f32 + 0.5 - cy;
				let r = ((dx / radius_px.max(1e-3)).powi(2) + (dy / radius_py.max(1e-3)).powi(2))
					.sqrt();
				if r >= 1.0 {
					continue;
				}
				let weight = (1.0 - r) * (1.0 - r);
				let mut dir_ss = Vec2::new(dx, dy);
				if dir_ss.length_squared() < 1e-4 {
					dir_ss = Vec2::X;
				} else {
					dir_ss = dir_ss.normalize();
				}
				let mut world = camera_right * dir_ss.x + Vec3::Y * (-dir_ss.y);
				world -= camera_forward * world.dot(camera_forward);
				world.y = 0.0;
				let world_xz = if world.length_squared() < 1e-6 {
					right_xz
				} else {
					world.xz().normalize_or(right_xz)
				};
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
		let mid = clip_to_uv(clip * occluder.center.extend(1.0));
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
