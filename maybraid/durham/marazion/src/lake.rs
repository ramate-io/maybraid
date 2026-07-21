//! Lake pocket water — [RFC-127 §3.1.3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3131-lake).
//!
//! Three-band footprint (center → edge):
//! - **Water** — bowl disc depressed below surface `W`
//! - **Rim** — flat plateau shelf slightly **above** `W` between water edge and plateau edge
//! - **Apron** — soft blend from plateau edge back to identity terrain
//!
//! Leaves must be ≈2×+ the water body so rim + apron fit without collapsing bands.

use crate::fill::WaterFill;
use crate::noise::{n01_at, n11_at};
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	CircleRegion, JerseyModulation, RegionAffineModulation, Region2D, RegionNoise,
};
use procedural_common::Bounds2;

/// Minimum water radius (world units); smaller budgets skip the stamp.
const MIN_WATER_RADIUS: f32 = 8.0;

#[derive(Debug, Clone, Copy)]
pub struct LakeParams {
	// ── Authoring knobs (tune these) ───────────────────────────────────────
	/// Rim shelf width as a fraction of the leaf radius budget.
	pub rim_frac: f32,
	/// Apron (blend-to-identity) width as a fraction of the leaf radius budget.
	pub apron_frac: f32,
	/// How far below the shelf anchor the water surface `W` sits (world units).
	pub water_sink: f32,
	/// How far the water difference bites into terrain (`h − undercut`).
	///
	/// This is the real shoreline bleed; keep it a bit above `rim_lift + water_sink`.
	pub terrain_undercut: f32,

	// ── Secondary / internal ───────────────────────────────────────────────
	/// Inward margin μ (world units) from cell boundary to the outer apron.
	pub mu: f32,
	/// Max centroid offset as a fraction of the smaller cell half-extent.
	pub centroid_jitter: f32,
	/// Shelf-anchor noise amplitude (world units).
	pub surface_noise_amp: f32,
	/// Bowl depth scale (world units).
	pub depth: f32,
	/// How far the rim shelf sits **above** the shelf anchor (world units).
	/// Bank height above water ≈ `rim_lift + water_sink`.
	pub rim_lift: f32,
	/// Horizontal softmask pad past the bowl, as a fraction of rim width.
	pub rim_bleed_frac: f32,
	/// SDF-relative fade past the fill disc edge.
	pub shore_fade: f32,
}

impl Default for LakeParams {
	fn default() -> Self {
		Self {
			// Authoring — start here:
			rim_frac: 0.28,
			apron_frac: 0.30,
			water_sink: 0.9,
			terrain_undercut: 2.5,

			mu: 12.0,
			centroid_jitter: 0.12,
			surface_noise_amp: 2.0,
			depth: 14.0,
			rim_lift: 1.25,
			// Modest horizontal pad; vertical undercut does the real bleed.
			rim_bleed_frac: 0.35,
			shore_fade: 2.0,
		}
	}
}

/// Radial band budget derived from a leaf short half-extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LakeBandBudget {
	pub water_radius: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	pub plateau_radius: f32,
	pub mu: f32,
}

impl LakeBandBudget {
	/// Budget water / rim / apron so the leaf is ≥2× the body diameter.
	///
	/// Returns `None` when the leaf cannot host a meaningful three-band lake.
	pub fn try_from_short_half(short_half: f32, params: LakeParams) -> Option<Self> {
		let s = short_half.max(0.0);
		if s < MIN_WATER_RADIUS * 2.0 {
			return None;
		}
		let mu = params.mu.min(s * 0.2).max(0.0);
		let available = (s - mu).max(0.0);
		if available < MIN_WATER_RADIUS * 2.0 {
			return None;
		}

		// Enforce leaf_short ≥ 2 · body_diameter ⇒ R_water ≤ short/4 = S/2.
		let max_water = (s * 0.5).min(available * 0.45);
		let rim = (available * params.rim_frac.clamp(0.05, 0.45))
			.max(available * 0.12)
			.min(available * 0.42);
		let apron = (available * params.apron_frac.clamp(0.05, 0.5))
			.max(available * 0.14)
			.min(available * 0.45);
		let water = (available - rim - apron).min(max_water);
		if water < MIN_WATER_RADIUS {
			return None;
		}
		// Re-check 2× body vs leaf: body diameter = 2·water, leaf = 2·S.
		if 2.0 * s < 2.0 * (2.0 * water) * 0.99 {
			return None;
		}

		Some(Self {
			water_radius: water,
			rim_width: rim,
			apron_width: apron,
			plateau_radius: water + rim,
			mu,
		})
	}
}

/// Lake stamp products for one pocket-water leaf.
#[derive(Debug, Clone)]
pub struct Lake {
	pub bounds: Bounds2,
	pub seed: u32,
	pub center: Vec2,
	pub water_radius: f32,
	pub plateau_radius: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	/// Fill disc radius (≥ [`Self::water_radius`]); bleeds into the rim shelf.
	pub fill_radius: f32,
	pub water_level: f32,
	pub modulations: Vec<JerseyModulation>,
	pub fills: Vec<WaterFill>,
}

impl Lake {
	/// Jittered lake centroid used by [`Self::from_bounds`] (for height prefetch).
	pub fn planned_center(bounds: Bounds2, seed: u32, params: LakeParams) -> Vec2 {
		let min = bounds.min;
		let max = bounds.max;
		let cell_c = Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5);
		let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
		let short_half = half.x.min(half.y).max(1.0);
		let Some(budget) = LakeBandBudget::try_from_short_half(short_half, params) else {
			return cell_c;
		};
		let outer = budget.plateau_radius + budget.apron_width;
		let lo = min + Vec2::splat(outer.max(budget.mu));
		let hi = max - Vec2::splat(outer.max(budget.mu));
		let ox = n11_at(seed, 0x1A7E_C001, min) * params.centroid_jitter * short_half;
		let oz = n11_at(seed, 0x1A7E_C002, min) * params.centroid_jitter * short_half;
		Vec2::new(
			(cell_c.x + ox).clamp(lo.x.min(hi.x), lo.x.max(hi.x)),
			(cell_c.y + oz).clamp(lo.y.min(hi.y), lo.y.max(hi.y)),
		)
	}

	/// Build a three-band lake, or an empty stamp when the leaf is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: LakeParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let min = bounds.min;
		let max = bounds.max;
		let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
		let short_half = half.x.min(half.y).max(1.0);
		let empty = |center: Vec2| Self {
			bounds,
			seed,
			center,
			water_radius: 0.0,
			plateau_radius: 0.0,
			rim_width: 0.0,
			apron_width: 0.0,
			fill_radius: 0.0,
			water_level: 0.0,
			modulations: Vec::new(),
			fills: Vec::new(),
		};

		let Some(budget) = LakeBandBudget::try_from_short_half(short_half, params) else {
			return empty(Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5));
		};

		let center = Self::planned_center(bounds, seed, params);
		let anchor = min;
		let base_h = height_at
			.map(|f| f(center.x, center.y))
			.unwrap_or(0.0);
		let shelf_anchor =
			base_h + n11_at(seed, 0x1A7E_50F1, anchor) * params.surface_noise_amp;
		let water_level = shelf_anchor - params.water_sink.max(0.0);
		let rim_level = shelf_anchor + params.rim_lift.max(0.0);
		let depth = params.depth * (0.65 + 0.7 * n01_at(seed, 0x1A7E_DE07, anchor));

		let water_r = budget.water_radius;
		let plateau_r = budget.plateau_radius;
		let rim_w = budget.rim_width;
		let apron_w = budget.apron_width.max(1.0);
		let bowl_fade = (rim_w * 0.25).max(0.5).min(water_r * 0.2);

		// Modest horizontal pad; terrain_undercut owns shoreline bleed.
		let rim_bleed = rim_w * params.rim_bleed_frac.max(0.0);
		let max_fill = plateau_r + apron_w - 0.5;
		let fill_r = (water_r + rim_bleed).min(max_fill).max(water_r);
		let fill_fade = params.shore_fade.max(1.0);

		let plateau_region = Region2D::Circle(CircleRegion {
			center,
			radius: plateau_r,
		});
		let water_region = Region2D::Circle(CircleRegion {
			center,
			radius: water_r,
		});
		let fill_region = Region2D::Circle(CircleRegion {
			center,
			radius: fill_r,
		});
		let noise = RegionNoise::from_seed(seed.wrapping_add(3), 0.016, water_r * 0.05);

		let plateau = JerseyModulation::Affine(
			RegionAffineModulation::new(plateau_region, 0.0, rim_level, 0.0, apron_w)
				.with_noise(noise.clone()),
		);
		// Bowl from raised shelf down past W.
		let bowl_depth = depth + (rim_level - water_level).max(0.0);
		let bowl = JerseyModulation::Affine(
			RegionAffineModulation::new(water_region, 1.0, -bowl_depth, 0.0, bowl_fade)
				.with_noise(noise.clone()),
		);

		let fill = WaterFill {
			region: fill_region,
			inner_radius: 0.0,
			outer_radius: fill_fade,
			noise: Some(noise),
			water_level,
			terrain_undercut: params.terrain_undercut.max(0.0),
		};

		Self {
			bounds,
			seed,
			center,
			water_radius: water_r,
			plateau_radius: plateau_r,
			rim_width: rim_w,
			apron_width: budget.apron_width,
			fill_radius: fill_r,
			water_level,
			modulations: vec![plateau, bowl],
			fills: vec![fill],
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, LakeParams::default(), None)
	}

	pub fn is_empty(&self) -> bool {
		self.modulations.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec2;

	fn apply_mods(mods: &[JerseyModulation], h: f32, x: f32, z: f32) -> f32 {
		let mut y = h;
		for m in mods {
			y = m.modify_elevation(y, x, z);
		}
		y
	}

	fn softmask_at(fill: &WaterFill, x: f32, z: f32) -> f32 {
		fill.region.softmask_weight(
			Vec2::new(x, z),
			fill.inner_radius,
			fill.outer_radius,
			fill.noise.as_ref(),
		)
	}

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 20.0, 20.0);
		let lake = Lake::from_bounds_default(bounds, 11);
		assert!(lake.is_empty());
		Ok(())
	}

	#[test]
	fn budget_enforces_two_x_body() -> anyhow::Result<()> {
		let short_half = 160.0;
		let budget = LakeBandBudget::try_from_short_half(short_half, LakeParams::default())
			.expect("budget");
		let leaf_short = 2.0 * short_half;
		let body_diameter = 2.0 * budget.water_radius;
		assert!(
			leaf_short + 1e-3 >= 2.0 * body_diameter,
			"leaf {leaf_short} should be ≥ 2× body {body_diameter}"
		);
		assert!(budget.rim_width > 0.0);
		assert!(budget.apron_width > 0.0);
		Ok(())
	}

	#[test]
	fn bowl_below_water_level() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let lake = Lake::from_bounds(bounds, 11, LakeParams::default(), Some(&|_, _| base));
		assert!(!lake.is_empty());
		let h = apply_mods(&lake.modulations, base, lake.center.x, lake.center.y);
		assert!(
			h < lake.water_level - 1.0,
			"bowl {h} should sit below surface {}",
			lake.water_level
		);
		Ok(())
	}

	#[test]
	fn rim_annulus_stays_near_surface() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let params = LakeParams::default();
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base));
		let mid_r = lake.water_radius + lake.rim_width * 0.5;
		let p = lake.center + Vec2::new(mid_r, 0.0);
		let h = apply_mods(&lake.modulations, base, p.x, p.y);
		let rim_level = lake.water_level + params.rim_lift + params.water_sink;
		assert!(
			(h - rim_level).abs() < 1.5,
			"rim {h} should stay near rim shelf {rim_level}"
		);
		assert!(
			h > lake.water_level + 0.25,
			"rim {h} should sit above water {}",
			lake.water_level
		);
		Ok(())
	}

	#[test]
	fn apron_blends_toward_identity() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let params = LakeParams::default();
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base));
		let apron_mid = lake.plateau_radius + lake.apron_width * 0.5;
		let p = lake.center + Vec2::new(apron_mid, 0.0);
		let h = apply_mods(&lake.modulations, base, p.x, p.y);
		let rim_level = lake.water_level + params.rim_lift + params.water_sink;
		let lo = base.min(rim_level);
		let hi = base.max(rim_level);
		assert!(
			h >= lo - 0.5 && h <= hi + 0.5,
			"apron {h} should sit between identity {base} and rim {rim_level}"
		);
		Ok(())
	}

	#[test]
	fn wet_softmask_inside_water_disc() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let lake = Lake::from_bounds_default(bounds, 11);
		let fill = lake.fills.first().expect("fill");
		let mid = lake.center;
		assert!(softmask_at(fill, mid.x, mid.y) < 0.25);
		let outside = lake.center
			+ Vec2::new(lake.plateau_radius + lake.apron_width + 20.0, 0.0);
		assert!(softmask_at(fill, outside.x, outside.y) >= 0.999);
		Ok(())
	}

	#[test]
	fn fill_pad_stays_near_bowl() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let params = LakeParams::default();
		let lake = Lake::from_bounds_default(bounds, 11);
		assert!(lake.fill_radius >= lake.water_radius);
		assert!(
			lake.fill_radius <= lake.water_radius + lake.rim_width + 1e-3,
			"horizontal pad should stay on the rim"
		);
		let fill = lake.fills.first().expect("fill");
		assert!(softmask_at(fill, lake.center.x, lake.center.y) < 0.25);
		let outside = lake.center
			+ Vec2::new(
				lake.plateau_radius + lake.apron_width + params.shore_fade + 5.0,
				0.0,
			);
		assert!(softmask_at(fill, outside.x, outside.y) >= 0.999);
		Ok(())
	}

	#[test]
	fn narrow_leaf_planned_center_does_not_panic() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 40.0, 200.0);
		let _ = Lake::planned_center(bounds, 3, LakeParams::default());
		let lake = Lake::from_bounds_default(bounds, 3);
		// May be empty (too narrow) but must not panic.
		let _ = lake.center;
		Ok(())
	}
}
