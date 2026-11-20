//! CPU mirror of `shading.wesl`.

use bevy::math::Vec3;

/// Linear RGB root-to-tip palette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VegetationPalette {
	pub root_rgb: Vec3,
	pub tip_rgb: Vec3,
}

impl VegetationPalette {
	/// Lush painterly meadow, matched to the terrain turf lush end.
	pub fn botw_meadow() -> Self {
		Self { root_rgb: Vec3::new(0.07, 0.30, 0.04), tip_rgb: Vec3::new(0.58, 0.84, 0.16) }
	}

	pub fn albedo(self, t: f32, variation: f32) -> Vec3 {
		grass_albedo(self, t, variation)
	}
}

pub fn grass_albedo(palette: VegetationPalette, t: f32, variation: f32) -> Vec3 {
	let tt = t.clamp(0.0, 1.0);
	let mid = palette.root_rgb.lerp(palette.tip_rgb, 0.42);
	let lower = palette.root_rgb.lerp(mid, (tt * 1.55).clamp(0.0, 1.0));
	let along = lower.lerp(palette.tip_rgb, smoothstep(0.42, 1.0, tt));
	along * (0.90 + 0.18 * variation.clamp(0.0, 1.0))
}

pub fn two_sided_wrap(normal: Vec3, light_dir: Vec3, wrap: f32) -> f32 {
	let n = normal.normalize_or_zero();
	let l = light_dir.normalize_or_zero();
	(n.dot(l).abs() * wrap + (1.0 - wrap)).clamp(0.0, 1.0)
}

pub fn painterly_lit(albedo: Vec3, normal: Vec3, light_dir: Vec3, t: f32) -> Vec3 {
	let wrap = two_sided_wrap(normal, light_dir, 0.42);
	let n = normal.normalize_or_zero();
	let l = light_dir.normalize_or_zero();
	let back = (-n.dot(l)).max(0.0) * 0.18;
	let sky = (n.y * 0.45 + 0.62).clamp(0.0, 1.0) * 0.28;
	let band0 = lerp(0.42, 1.0, smoothstep(0.14, 0.28, wrap));
	let band1 = lerp(band0, 1.12, smoothstep(0.58, 0.78, wrap));
	let root_dark = lerp(0.58, 1.0, smoothstep(0.0, 0.34, t.clamp(0.0, 1.0)));
	albedo * (band1 * root_dark + back + sky)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn root_is_darker_than_tip() -> anyhow::Result<()> {
		let pal = VegetationPalette::botw_meadow();
		let root = painterly_lit(pal.albedo(0.0, 0.5), Vec3::Y, Vec3::new(0.4, 0.8, 0.3), 0.0);
		let tip = painterly_lit(pal.albedo(1.0, 0.5), Vec3::Y, Vec3::new(0.4, 0.8, 0.3), 1.0);
		assert!(root.length() < tip.length());
		Ok(())
	}

	#[test]
	fn backlight_lifts_facing_away() -> anyhow::Result<()> {
		let albedo = Vec3::splat(0.4);
		let n = Vec3::new(0.0, 0.2, 1.0);
		let l = Vec3::new(0.0, 0.2, -1.0);
		let lit = painterly_lit(albedo, n, l, 0.6);
		assert!(lit.length() > 0.05);
		Ok(())
	}
}
