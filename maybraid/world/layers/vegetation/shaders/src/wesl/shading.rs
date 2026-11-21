//! CPU mirror of `shading.wesl`.

use bevy::math::{Vec2, Vec3};

use super::hash::{hash_u32, hash_u32_to_unit, value_noise_2};

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

	/// Layered tree canopy: dark cool interior, warm sunlit rim.
	pub fn botw_canopy_crown() -> Self {
		Self { root_rgb: Vec3::new(0.045, 0.20, 0.10), tip_rgb: Vec3::new(0.46, 0.72, 0.16) }
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

/// Inner/lower shells stay cooler and darker; outer/top shells go warm and light.
pub fn crown_layer_albedo(
	palette: VegetationPalette,
	height_frac: f32,
	radial_frac: f32,
	shell: f32,
	variation: f32,
) -> Vec3 {
	let cool = palette.root_rgb * Vec3::new(0.52, 0.78, 1.38);
	let warm = palette.tip_rgb * Vec3::new(1.28, 1.10, 0.68);
	let along = (height_frac * 0.42 + radial_frac.clamp(0.0, 1.0) * 0.28 + shell.clamp(0.0, 1.0) * 0.38)
		.clamp(0.0, 1.0);
	let mixed = cool.lerp(warm, smoothstep(0.10, 0.88, along));
	mixed * (0.84 + 0.28 * variation.clamp(0.0, 1.0))
}

/// Billowy canopy clump: height warmth plus a cooler, darker underside.
pub fn canopy_clump_albedo(
	warm_rgb: Vec3,
	cool_rgb: Vec3,
	height_frac: f32,
	normal: Vec3,
	clump_variation: f32,
) -> Vec3 {
	let sunlit = crown_albedo(warm_rgb, cool_rgb, height_frac, clump_variation);
	let lift = smoothstep(-0.22, 0.48, normal.normalize_or_zero().y);
	sunlit * Vec3::new(0.28, 0.40, 0.58).lerp(Vec3::ONE, lift)
}

/// Ambient-occlusion-like darkening for the crown core and downward faces.
pub fn canopy_interior_shade(height_frac: f32, radial_frac: f32, shell: f32, normal: Vec3) -> f32 {
	let n = normal.normalize_or_zero();
	let under = 1.0 - smoothstep(-0.12, 0.40, n.y);
	let core = (1.0 - shell.clamp(0.0, 1.0)) * (1.0 - radial_frac.clamp(0.0, 1.0) * 0.52);
	let low = 1.0 - height_frac.clamp(0.0, 1.0);
	lerp(0.26, 1.0, 1.0 - (under * 0.58 + core * 0.64 + low * 0.20).clamp(0.0, 1.0))
}

/// CPU mirror of hashed in-card leaf silhouettes.
pub struct LeafCluster;

impl LeafCluster {
	pub fn mask(uv: Vec2, seed: u32, detail: f32) -> f32 {
		leaf_cluster_mask(uv, seed, detail)
	}

	pub fn tint(uv: Vec2, seed: u32) -> f32 {
		leaf_cluster_tint(uv, seed)
	}

	pub fn keep(alpha: f32, pixel: Vec2, seed: u32, detail: f32) -> bool {
		leaf_cluster_keep(alpha, pixel, seed, detail)
	}

	pub fn alpha_for_style(uv: Vec2, seed: u32, detail: f32, style: u32) -> f32 {
		leaf_cluster_alpha_for_style(uv, seed, detail, style)
	}

	pub fn keep_for_style(alpha: f32, pixel: Vec2, seed: u32, detail: f32, style: u32) -> bool {
		leaf_cluster_keep_for_style(alpha, pixel, seed, detail, style)
	}

	pub fn outward_normal(geometric: Vec3, clump_out: Vec3, uv: Vec2, detail: f32) -> Vec3 {
		leaf_cluster_outward_normal(geometric, clump_out, uv, detail)
	}
}

fn hashed_leaf_blob(p: Vec2, cell: bevy::math::IVec2, seed: u32) -> f32 {
	let packed = (cell.x.wrapping_add(173) as u32).wrapping_mul(374_761_393)
		^ (cell.y.wrapping_add(91) as u32).wrapping_mul(668_265_263);
	let h0 = hash_u32(seed ^ hash_u32(packed));
	let h1 = hash_u32(h0 ^ 0x85ebca6b);
	let ox = hash_u32_to_unit(h1);
	let oy = hash_u32_to_unit(h1 ^ 0xc2b2ae35);
	let stretch = 0.70 + 0.24 * hash_u32_to_unit(h1 ^ 0x27d4eb2d);
	let center = Vec2::new(cell.x as f32 + ox * 0.74, cell.y as f32 + oy * 0.74);
	let delta = p - center;
	let v = (delta.y * 0.55 + 0.5).clamp(0.0, 1.0);
	let petiole = smoothstep(0.0, 0.22, v);
	let tip = 1.0 - 0.55 * smoothstep(0.48, 1.0, v);
	let width = (petiole * tip).max(0.16);
	let q = Vec2::new(delta.x / (0.38 * stretch * width), delta.y / (0.52 * stretch));
	1.0 - smoothstep(0.58, 1.0, q.length())
}

pub fn leaf_cluster_mask(uv: Vec2, seed: u32, detail: f32) -> f32 {
	let d = detail.clamp(0.0, 1.0);
	let cells_v = lerp(2.2, 6.4, d);
	let cells_u = lerp(1.8, 5.2, d);
	let p = Vec2::new((uv.x * 0.5 + 0.5) * cells_u, uv.y.clamp(0.0, 1.0) * cells_v);
	let base = bevy::math::IVec2::new(p.x.floor() as i32, p.y.floor() as i32);
	let mut occ = 0.0_f32;
	for y in -1..=1 {
		for x in -1..=1 {
			occ = occ.max(hashed_leaf_blob(p, base + bevy::math::IVec2::new(x, y), seed));
		}
	}
	let filled = (occ * lerp(1.55, 1.12, d)).clamp(0.0, 1.0);
	lerp(1.0, filled, d)
}

pub fn leaf_cluster_tint(uv: Vec2, seed: u32) -> f32 {
	let cell = bevy::math::IVec2::new(
		((uv.x * 0.5 + 0.5) * 5.0).floor() as i32,
		(uv.y.clamp(0.0, 1.0) * 6.0).floor() as i32,
	);
	let packed = (cell.x.wrapping_add(19) as u32).wrapping_mul(374_761_393)
		^ (cell.y.wrapping_add(7) as u32).wrapping_mul(668_265_263);
	hash_u32_to_unit(seed ^ hash_u32(packed) ^ 0xa0761d65)
}

pub fn leaf_cluster_keep(alpha: f32, pixel: Vec2, seed: u32, detail: f32) -> bool {
	if detail < 0.08 {
		return true;
	}
	if alpha < 0.06 {
		return false;
	}
	if alpha > 0.92 {
		return true;
	}
	let hx = hash_u32(seed ^ (pixel.x as u32).wrapping_mul(1973) ^ (pixel.y as u32).wrapping_mul(9277));
	alpha > hash_u32_to_unit(hx) * 0.78 + 0.08
}

pub const LEAF_CLOSE_SMOOTH: u32 = 0;
pub const LEAF_CLOSE_DITHER: u32 = 1;
pub const LEAF_CLOSE_CRISP: u32 = 2;

pub fn leaf_close_uses_cluster(style: u32) -> bool {
	style != LEAF_CLOSE_SMOOTH
}

pub fn leaf_cluster_alpha_for_style(uv: Vec2, seed: u32, detail: f32, style: u32) -> f32 {
	if style == LEAF_CLOSE_SMOOTH {
		1.0
	} else {
		leaf_cluster_mask(uv, seed, detail)
	}
}

pub fn leaf_cluster_keep_for_style(
	alpha: f32,
	pixel: Vec2,
	seed: u32,
	detail: f32,
	style: u32,
) -> bool {
	if style == LEAF_CLOSE_SMOOTH {
		return alpha >= 0.04;
	}
	if style == LEAF_CLOSE_CRISP {
		if detail < 0.08 {
			return true;
		}
		return alpha >= 0.48;
	}
	leaf_cluster_keep(alpha, pixel, seed, detail)
}

pub fn leaf_cluster_outward_normal(geometric: Vec3, clump_out: Vec3, uv: Vec2, detail: f32) -> Vec3 {
	let toward = (clump_out + Vec3::new(0.0, 0.12, 0.0)).normalize_or(Vec3::Y);
	let edge = uv.x.abs().clamp(0.0, 1.0);
	let amt = detail.clamp(0.0, 1.0) * lerp(0.48, 0.18, edge);
	geometric.normalize_or(Vec3::Y).lerp(toward, amt).normalize_or(Vec3::Y)
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

	#[test]
	fn canopy_clump_underside_is_cooler_than_the_lit_rim() -> anyhow::Result<()> {
		let pal = VegetationPalette::botw_canopy_crown();
		let top = canopy_clump_albedo(pal.tip_rgb, pal.root_rgb, 0.92, Vec3::Y, 0.4);
		let under = canopy_clump_albedo(pal.tip_rgb, pal.root_rgb, 0.92, -Vec3::Y, 0.4);
		assert!(top.x + top.y > under.x + under.y);
		Ok(())
	}

	#[test]
	fn canopy_interior_is_darker_under_and_inside() -> anyhow::Result<()> {
		let rim = canopy_interior_shade(0.9, 0.9, 1.0, Vec3::Y);
		let core = canopy_interior_shade(0.2, 0.1, 0.0, -Vec3::Y);
		assert!(rim > core + 0.2);
		Ok(())
	}

	#[test]
	fn leaf_close_styles_keep_or_cut_as_authored() -> anyhow::Result<()> {
		let uv = Vec2::new(-0.7, 0.15);
		let pixel = Vec2::new(12.0, 9.0);
		let seed = 19u32;
		assert!((LeafCluster::alpha_for_style(uv, seed, 1.0, LEAF_CLOSE_SMOOTH) - 1.0).abs() < 1e-4);
		assert!(LeafCluster::keep_for_style(0.2, pixel, seed, 1.0, LEAF_CLOSE_SMOOTH));
		let dither_a = LeafCluster::alpha_for_style(uv, seed, 1.0, LEAF_CLOSE_DITHER);
		let crisp_a = LeafCluster::alpha_for_style(uv, seed, 1.0, LEAF_CLOSE_CRISP);
		assert!((dither_a - crisp_a).abs() < 1e-5);
		assert!(!LeafCluster::keep_for_style(0.2, pixel, seed, 1.0, LEAF_CLOSE_CRISP));
		assert!(LeafCluster::keep_for_style(0.8, pixel, seed, 1.0, LEAF_CLOSE_CRISP));
		Ok(())
	}

	#[test]
	fn leaf_cluster_near_has_holes_far_is_solid() -> anyhow::Result<()> {
		let seed = 11u32;
		let mut min_near = 1.0_f32;
		let mut max_near = 0.0_f32;
		for i in 0..48u32 {
			let u = (i % 8) as f32 / 7.0 * 2.0 - 1.0;
			let v = (i / 8) as f32 / 5.0;
			let a = LeafCluster::mask(Vec2::new(u, v), seed, 1.0);
			min_near = min_near.min(a);
			max_near = max_near.max(a);
			assert!((LeafCluster::mask(Vec2::new(u, v), seed, 0.0) - 1.0).abs() < 1e-4);
		}
		assert!(max_near > 0.7, "near occupancy {max_near}");
		assert!(min_near < 0.45, "near should punch holes, got min {min_near}");
		Ok(())
	}

	#[test]
	fn leaf_cluster_tint_varies_across_the_card() -> anyhow::Result<()> {
		let a = LeafCluster::tint(Vec2::new(-0.8, 0.1), 4);
		let b = LeafCluster::tint(Vec2::new(0.7, 0.8), 4);
		assert!((a - b).abs() > 0.02);
		Ok(())
	}

	#[test]
	fn crown_layer_is_cooler_inside_and_warmer_outside() -> anyhow::Result<()> {
		let pal = VegetationPalette::botw_scrub_crown();
		let inner = crown_layer_albedo(pal, 0.15, 0.10, 0.0, 0.4);
		let outer = crown_layer_albedo(pal, 0.88, 0.85, 1.0, 0.4);
		assert!(outer.x + outer.y > inner.x + inner.y);
		assert!(inner.z / (inner.x + 1e-4) > outer.z / (outer.x + 1e-4) * 0.85);
		Ok(())
	}
}
