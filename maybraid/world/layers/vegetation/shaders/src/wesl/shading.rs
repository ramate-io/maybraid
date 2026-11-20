//! CPU mirror of `shading.wesl`.

use bevy::math::{Vec2, Vec3};

use super::hash::{hash_u32_to_unit, value_noise_2};

/// Linear RGB root-to-tip palette.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VegetationPalette {
	pub root_rgb: Vec3,
	pub tip_rgb: Vec3,
}

impl VegetationPalette {
	/// Lush painterly meadow: cooler dark roots, warmer light tips.
	pub fn botw_meadow() -> Self {
		Self { root_rgb: Vec3::new(0.045, 0.16, 0.07), tip_rgb: Vec3::new(0.42, 0.68, 0.18) }
	}

	/// Warm bark for small woody stems.
	pub fn botw_wood() -> Self {
		Self { root_rgb: Vec3::new(0.22, 0.12, 0.07), tip_rgb: Vec3::new(0.46, 0.30, 0.16) }
	}

	/// Cool interior / warm rim foliage for BotW-like scrub crowns.
	pub fn botw_scrub_crown() -> Self {
		Self { root_rgb: Vec3::new(0.07, 0.26, 0.11), tip_rgb: Vec3::new(0.40, 0.64, 0.16) }
	}

	pub fn albedo(self, t: f32, variation: f32) -> Vec3 {
		grass_albedo(self, t, variation)
	}

	pub fn varied(self, world_xz: Vec2, plant_id: u32) -> Self {
		grass_variation_palette(self.tip_rgb, world_xz, plant_id)
	}
}

/// Low-frequency meadow colour field. Patches drift across tens of metres.
pub fn meadow_color_field(world_xz: Vec2) -> f32 {
	let n0 = value_noise_2(world_xz * 0.085);
	let n1 = value_noise_2(world_xz * 0.031 + Vec2::new(17.2, 9.4));
	(n0 * 0.62 + n1 * 0.38).clamp(0.0, 1.0)
}

/// Per-blade / per-patch palette driven by stable hashes and [`meadow_color_field`].
pub fn grass_variation_palette(
	species_mid: Vec3,
	world_xz: Vec2,
	plant_id: u32,
) -> VegetationPalette {
	let field = meadow_color_field(world_xz);
	let u = hash_u32_to_unit(plant_id ^ 0x7feb352d);
	let v = hash_u32_to_unit(plant_id ^ 0x27d4eb2d);
	let deep = Vec3::new(0.05, 0.22, 0.08);
	let lush = Vec3::new(0.12, 0.40, 0.10);
	let yellow = Vec3::new(0.38, 0.52, 0.10);
	let gold = Vec3::new(0.52, 0.44, 0.12);
	let yellow_amt = (field * 0.82 + u * 0.38).clamp(0.0, 1.0);
	let mut mid = deep.lerp(lush.lerp(yellow, yellow_amt), 0.38 + 0.55 * field);
	mid = mid.lerp(species_mid, 0.16);
	if v > 0.93 {
		mid = mid.lerp(gold, 0.78);
	}
	let root_rgb = mid * Vec3::new(0.32, 0.48, 0.62);
	let tip_rgb = (mid * Vec3::new(1.38, 1.22, 0.78) + Vec3::new(0.06, 0.10, 0.02))
		.clamp(Vec3::ZERO, Vec3::ONE);
	VegetationPalette { root_rgb, tip_rgb }
}

pub fn grass_albedo(palette: VegetationPalette, t: f32, variation: f32) -> Vec3 {
	let tt = t.clamp(0.0, 1.0);
	let mid = palette.root_rgb.lerp(palette.tip_rgb, 0.38);
	let lower = palette.root_rgb.lerp(mid, (tt * 1.45).clamp(0.0, 1.0));
	let along = lower.lerp(palette.tip_rgb, smoothstep(0.38, 1.0, tt));
	along * (0.88 + 0.20 * variation.clamp(0.0, 1.0))
}

pub fn two_sided_wrap(normal: Vec3, light_dir: Vec3, wrap: f32) -> f32 {
	let n = normal.normalize_or_zero();
	let l = light_dir.normalize_or_zero();
	(n.dot(l).abs() * wrap + (1.0 - wrap)).clamp(0.0, 1.0)
}

pub fn painterly_lit(albedo: Vec3, normal: Vec3, light_dir: Vec3, t: f32) -> Vec3 {
	let wrap = two_sided_wrap(normal, light_dir, 0.38);
	let n = normal.normalize_or_zero();
	let l = light_dir.normalize_or_zero();
	let back = (-n.dot(l)).max(0.0) * 0.16;
	let sky = (n.y * 0.40 + 0.66).clamp(0.0, 1.0) * 0.30;
	let band0 = lerp(0.48, 1.0, smoothstep(0.10, 0.40, wrap));
	let band1 = lerp(band0, 1.08, smoothstep(0.55, 0.92, wrap));
	let root_dark = lerp(0.52, 1.0, smoothstep(0.0, 0.38, t.clamp(0.0, 1.0)));
	albedo * (band1 * root_dark + back + sky)
}

pub fn wood_albedo(palette: VegetationPalette, t: f32, variation: f32) -> Vec3 {
	let along = palette.root_rgb.lerp(palette.tip_rgb, (t * 0.55 + 0.12).clamp(0.0, 1.0));
	along * (0.78 + 0.28 * variation.clamp(0.0, 1.0))
}

pub fn leaf_painterly_lit(albedo: Vec3, normal: Vec3, light_dir: Vec3, t: f32) -> Vec3 {
	let wrap = two_sided_wrap(normal, light_dir, 0.62);
	let n = normal.normalize_or_zero();
	let l = light_dir.normalize_or_zero();
	let back = (-n.dot(l)).max(0.0) * 0.34;
	let sky = (n.y * 0.55 + 0.50).clamp(0.0, 1.0) * 0.34;
	let band0 = lerp(0.38, 1.05, smoothstep(0.10, 0.32, wrap));
	let band1 = lerp(band0, 1.18, smoothstep(0.55, 0.82, wrap));
	let tip_lift = lerp(0.78, 1.0, smoothstep(0.0, 0.45, t.clamp(0.0, 1.0)));
	albedo * (band1 * tip_lift + back + sky)
}

pub fn crown_albedo(warm_rgb: Vec3, cool_rgb: Vec3, height_frac: f32, variation: f32) -> Vec3 {
	let along = cool_rgb.lerp(warm_rgb, smoothstep(0.18, 0.88, height_frac.clamp(0.0, 1.0)));
	along * (0.88 + 0.22 * variation.clamp(0.0, 1.0))
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
		assert!(pal.root_rgb.x < pal.tip_rgb.x || pal.root_rgb.z > pal.tip_rgb.z * 0.2);
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

	#[test]
	fn meadow_field_shifts_hue_across_the_patch() -> anyhow::Result<()> {
		let field_a = meadow_color_field(Vec2::new(4.0, 4.0));
		let field_b = meadow_color_field(Vec2::new(40.0, -22.0));
		assert!((field_a - field_b).abs() > 0.04);
		let a = grass_variation_palette(Vec3::new(0.2, 0.5, 0.1), Vec2::new(4.0, 4.0), 11);
		let b = grass_variation_palette(Vec3::new(0.2, 0.5, 0.1), Vec2::new(40.0, -22.0), 11);
		assert!((a.tip_rgb - b.tip_rgb).length() > 0.01);
		Ok(())
	}

	#[test]
	fn per_blade_hash_spreads_yellow_and_deep_greens() -> anyhow::Result<()> {
		let mut min_g = f32::MAX;
		let mut max_r: f32 = 0.0;
		for i in 0..64u32 {
			let pal = grass_variation_palette(Vec3::new(0.2, 0.45, 0.1), Vec2::new(8.0, 9.0), i);
			min_g = min_g.min(pal.tip_rgb.y);
			max_r = max_r.max(pal.tip_rgb.x);
		}
		assert!(max_r - min_g > 0.04 || max_r > 0.35);
		Ok(())
	}

	#[test]
	fn crown_gradient_is_warmer_at_the_top() -> anyhow::Result<()> {
		let cool = Vec3::new(0.08, 0.30, 0.14);
		let warm = Vec3::new(0.52, 0.78, 0.16);
		let low = crown_albedo(warm, cool, 0.1, 0.4);
		let high = crown_albedo(warm, cool, 0.9, 0.4);
		assert!(high.x + high.y > low.x + low.y);
		Ok(())
	}

	#[test]
	fn leaf_lit_keeps_backlight() -> anyhow::Result<()> {
		let albedo = Vec3::new(0.3, 0.5, 0.15);
		let n = Vec3::new(0.0, 0.1, 1.0);
		let l = Vec3::new(0.0, 0.1, -1.0);
		let lit = leaf_painterly_lit(albedo, n, l, 0.7);
		assert!(lit.length() > 0.05);
		Ok(())
	}
}
