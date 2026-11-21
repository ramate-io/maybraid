//! View-dependent grass parting: settings, occluders, and a cheap raster mask.

use bevy::math::{Mat4, Vec2, Vec3, Vec3Swizzles, Vec4};
use bevy::prelude::*;
use vegetation_shaders::wesl::{
	GrassPartingUniform, MAX_PARTING_OCCLUDERS, PARTING_MODE_DEPTH, PARTING_MODE_OFF,
	PARTING_MODE_RASTER, PARTING_MODE_WORLD,
};

/// How blades sample occluders. World is the production default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum GrassPartingMode {
	Off,
	/// Sample the camera depth prepass at the projected tip (arbitrary opaque geometry).
	Depth,
	/// World-XZ occupancy disc around tagged occluders (stamp_scale × radius).
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
				if self.mode == GrassPartingMode::Raster {
					self.stamp_scale.max(0.05)
				} else {
					self.kernel_px.max(1.0)
				},
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

/// Playground / capture toggles. Raster displacement does not read this.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GrassPartingDebug {
	/// Draw the world-XZ occupancy disc as a magenta ground overlay.
	pub mask_overlay: bool,
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
pub const PARTING_MASK_HEIGHT: u32 = 256;
/// Square world-XZ footprint around the camera (metres on a side).
pub const PARTING_MASK_EXTENT_M: f32 = 16.0;

/// World-XZ occupancy disc. RG = away direction × weight, B = occupancy,
/// A unused. The vertex shader evaluates the same disc from packed occluders;
/// this texture is the debug overlay / CPU alignment source.
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

	/// Magenta occupancy preview (straight alpha) for the debug ground overlay.
	pub fn occupancy_preview_rgba8(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
		for pixel in &self.pixels {
			let occupancy = pixel[2].clamp(0.0, 1.0);
			let alpha = (occupancy * 210.0) as u8;
			bytes.extend_from_slice(&[220, 36, 255, alpha]);
		}
		bytes
	}

	/// Splat circular occupancy discs in world XZ around `origin_xz`.
	pub fn splat(
		&mut self,
		occluders: &[ExtractedPartingOccluder],
		origin_xz: Vec2,
		settings: GrassPartingSettings,
	) {
		self.clear();
		for occluder in occluders.iter().take(MAX_PARTING_OCCLUDERS) {
			self.splat_one(occluder, origin_xz, settings);
		}
	}

	fn splat_one(
		&mut self,
		occluder: &ExtractedPartingOccluder,
		origin_xz: Vec2,
		settings: GrassPartingSettings,
	) {
		let centre = occluder.center.xz();
		let radius_m = settings.stamp_radius_m(occluder.radius_m);
		let uv = world_xz_to_mask_uv(centre, origin_xz);
		if !(0.0..=1.0).contains(&uv.x) || !(0.0..=1.0).contains(&uv.y) {
			return;
		}
		let texel_m = PARTING_MASK_EXTENT_M / self.width.max(1) as f32;
		let radius_px = (radius_m / texel_m.max(1e-4)).max(1.0);
		let cx = uv.x * self.width as f32;
		let cy = uv.y * self.height as f32;
		let min_x = (cx - radius_px).floor().max(0.0) as u32;
		let max_x = (cx + radius_px).ceil().min(self.width as f32 - 1.0) as u32;
		let min_y = (cy - radius_px).floor().max(0.0) as u32;
		let max_y = (cy + radius_px).ceil().min(self.height as f32 - 1.0) as u32;
		for y in min_y..=max_y {
			for x in min_x..=max_x {
				let world = self.texel_world_xz(x, y, origin_xz);
				let delta = world - centre;
				let r = delta.length() / radius_m.max(1e-3);
				if r >= 1.0 {
					continue;
				}
				let weight = stamp_falloff(r);
				let away = if delta.length_squared() < 1e-8 { Vec2::X } else { delta.normalize() };
				let i = (y * self.width + x) as usize;
				self.pixels[i][0] += away.x * weight;
				self.pixels[i][1] += away.y * weight;
				self.pixels[i][2] += weight;
			}
		}
	}

	pub fn texel_world_xz(&self, x: u32, y: u32, origin_xz: Vec2) -> Vec2 {
		let uv = Vec2::new(
			(x as f32 + 0.5) / self.width.max(1) as f32,
			(y as f32 + 0.5) / self.height.max(1) as f32,
		);
		origin_xz + (uv - Vec2::splat(0.5)) * PARTING_MASK_EXTENT_M
	}

	/// Occupancy-weighted world XZ (mask-centre regression). A flat inner core
	/// makes a raw peak texel wander; the centroid stays on the occluder.
	pub fn occupancy_centroid_world_xz(&self, origin_xz: Vec2) -> Option<Vec2> {
		let mut acc = Vec2::ZERO;
		let mut weight = 0.0f32;
		for y in 0..self.height {
			for x in 0..self.width {
				let occupancy = self.pixels[(y * self.width + x) as usize][2];
				if occupancy <= 0.05 {
					continue;
				}
				acc += self.texel_world_xz(x, y, origin_xz) * occupancy;
				weight += occupancy;
			}
		}
		(weight > 1e-4).then_some(acc / weight)
	}

	#[cfg(test)]
	fn occupancy_count(&self, min_weight: f32) -> usize {
		self.pixels.iter().filter(|pixel| pixel[2] > min_weight).count()
	}

	#[cfg(test)]
	fn occupancy_at_world(&self, world_xz: Vec2, origin_xz: Vec2) -> f32 {
		let uv = world_xz_to_mask_uv(world_xz, origin_xz);
		if !(0.0..1.0).contains(&uv.x) || !(0.0..1.0).contains(&uv.y) {
			return 0.0;
		}
		let x = (uv.x * self.width as f32).floor() as u32;
		let y = (uv.y * self.height as f32).floor() as u32;
		self.pixels[(y.min(self.height - 1) * self.width + x.min(self.width - 1)) as usize][2]
	}
}

/// Soft occupancy: full in the inner ~25% of the stamp, fading to the edge.
fn stamp_falloff(r: f32) -> f32 {
	1.0 - smoothstep(0.25, 1.0, r)
}

pub fn world_xz_to_mask_uv(world_xz: Vec2, origin_xz: Vec2) -> Vec2 {
	(world_xz - origin_xz) / PARTING_MASK_EXTENT_M + Vec2::splat(0.5)
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = ((x - edge0) / (edge1 - edge0).max(1e-6)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn raster_splat_points_away_from_the_disc_centre() -> anyhow::Result<()> {
		let mut mask = PartingMask::new(64, 64);
		let origin = Vec2::new(0.0, 4.0);
		let occluder = ExtractedPartingOccluder {
			center: Vec3::new(0.0, 0.9, 4.0),
			radius_m: 0.4,
			height_m: 1.8,
		};
		mask.splat(
			&[occluder],
			origin,
			GrassPartingSettings::default().with_mode(GrassPartingMode::Raster),
		);
		let left = mask.occupancy_at_world(origin + Vec2::new(-0.4, 0.0), origin);
		let right = mask.occupancy_at_world(origin + Vec2::new(0.4, 0.0), origin);
		assert!(left > 0.2 && right > 0.2, "expected occupancy both sides, L {left} R {right}");
		let left_uv = world_xz_to_mask_uv(origin + Vec2::new(-0.4, 0.0), origin);
		let right_uv = world_xz_to_mask_uv(origin + Vec2::new(0.4, 0.0), origin);
		let lx = (left_uv.x * 64.0).floor() as u32;
		let ly = (left_uv.y * 64.0).floor() as u32;
		let rx = (right_uv.x * 64.0).floor() as u32;
		let ry = (right_uv.y * 64.0).floor() as u32;
		let left_px = mask.pixels[(ly * 64 + lx) as usize];
		let right_px = mask.pixels[(ry * 64 + rx) as usize];
		assert!(
			left_px[0] < 0.0 && right_px[0] > 0.0,
			"stamp should peel away in X, left {left_px:?} right {right_px:?}"
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
		let origin = Vec2::ZERO;
		let occluder = ExtractedPartingOccluder {
			center: Vec3::new(0.0, 0.9, 0.0),
			radius_m: 0.4,
			height_m: 1.8,
		};
		let splat = |scale: f32| {
			let mut mask = PartingMask::new(64, 64);
			mask.splat(
				&[occluder],
				origin,
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

	#[test]
	fn raster_pack_puts_stamp_scale_in_extras_z() -> anyhow::Result<()> {
		let uniform = GrassPartingSettings::default()
			.with_mode(GrassPartingMode::Raster)
			.with_stamp_scale(2.5)
			.pack(Mat4::IDENTITY, Vec3::ZERO, Vec3::X, Vec3::Z, &[]);
		assert!((uniform.extras.z - 2.5).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn raster_mask_centre_maps_to_occluder_world_xz() -> anyhow::Result<()> {
		let camera = Vec3::new(9.62, 5.9, 10.00);
		let occluder = ExtractedPartingOccluder {
			center: Vec3::new(9.62, 0.9, 10.54),
			radius_m: 0.4,
			height_m: 1.8,
		};
		let settings = GrassPartingSettings::default()
			.with_mode(GrassPartingMode::Raster)
			.with_stamp_scale(2.5);
		let mut mask = PartingMask::default_size();
		mask.splat(&[occluder], camera.xz(), settings);
		let at_centre = mask.occupancy_at_world(occluder.center.xz(), camera.xz());
		assert!(at_centre > 0.9, "occupancy at the occluder XZ should be ~1, got {at_centre}");
		let centre = mask
			.occupancy_centroid_world_xz(camera.xz())
			.expect("stamp should write occupancy");
		let dist = centre.distance(occluder.center.xz());
		assert!(
			dist < 0.12,
			"occupancy centroid {centre:?} should sit on occluder {:?} (dist {dist})",
			occluder.center.xz()
		);
		let radius_m = settings.stamp_radius_m(occluder.radius_m);
		assert!((radius_m - 1.0).abs() < 1e-5);
		let outside = mask.occupancy_at_world(occluder.center.xz() + Vec2::X * 1.15, camera.xz());
		assert!(outside < 0.05, "occupancy past 2.5× radius should be ~0, got {outside}");
		Ok(())
	}
}
