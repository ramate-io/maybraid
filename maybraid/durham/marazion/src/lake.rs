//! Lake pocket water — [RFC-127 §3.1.3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3131-lake).
//!
//! Three-band footprint (center → edge):
//! - **Water** — bowl disc depressed below surface `W`
//! - **Rim** — flat plateau shelf slightly **above** `W` between water edge and plateau edge
//! - **Apron** — soft blend from plateau edge back to identity terrain
//!
//! Leaves must be ≈2×+ the water body so rim + apron fit without collapsing bands.

use crate::fill::WaterFill;
use crate::noise::{n01_at, n01_freq, n11_at};
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	CircleRegion, JerseyModulation, Region2D, RegionAffineModulation, RegionNoise,
};
use procedural_common::Bounds2;

/// Minimum water radius (world units); smaller budgets skip the stamp.
const MIN_WATER_RADIUS: f32 = 8.0;

/// Salt for per-leaf water-radius undershoot.
const WATER_SCALE_SALT: u32 = 0x1A7E_512E;
/// Salt for per-leaf rim-width undershoot.
const RIM_WIDTH_SALT: u32 = 0x1A7E_71B7;

/// Authoring knobs for a Marazion lake stamp.
///
/// Grouped by what they control: footprint budget → per-leaf scales → vertical
/// levels → bowl depth → shore outline → apron outline → rim height.
#[derive(Debug, Clone, Copy)]
pub struct LakeParams {
	// ── Footprint budget ───────────────────────────────────────────────────
	/// Rim shelf width as a fraction of the leaf radius budget.
	pub rim_frac: f32,
	/// Apron (blend-to-identity) width as a fraction of the leaf radius budget.
	pub apron_frac: f32,
	/// Inward margin μ (world units) from cell boundary to the outer apron.
	pub mu: f32,
	/// Max centroid offset as a fraction of the smaller cell half-extent.
	pub centroid_jitter: f32,

	// ── Per-leaf water scale (fraction of leftover after rim+apron claim) ───
	/// Max water radius scale (`1.0` = full leftover).
	pub water_scale: f32,
	/// Min water radius scale (paired with [`Self::water_scale`] via leaf noise).
	pub water_scale_min: f32,
	/// Spatial frequency for the water-scale draw.
	pub water_scale_freq: f32,

	// ── Per-leaf rim width ──────────────────────────────────────────────────
	/// Min rim width as a fraction of the claimed rim budget.
	pub rim_width_min: f32,
	/// Spatial frequency for the rim-width draw.
	pub rim_width_freq: f32,

	// ── Vertical levels ────────────────────────────────────────────────────
	/// How far below the shelf anchor the water surface `W` sits (world units).
	pub water_sink: f32,
	/// How far the rim shelf sits **above** the shelf anchor (world units).
	pub rim_lift: f32,
	/// How far the water difference bites into terrain (`h − undercut`).
	pub terrain_undercut: f32,
	/// Per-leaf jitter on the shelf anchor height that sets `W` and rim base.
	pub shelf_amp: f32,
	/// How many pre-watershed heights to sample on a ring around the centroid
	/// (at mid-rim radius) when setting the shelf anchor. `1` = centroid only.
	pub shelf_sample_count: u8,

	// ── Bowl depth ─────────────────────────────────────────────────────────
	/// Bowl depth scale (world units).
	pub depth: f32,
	/// Fraction of depth reserved for the nested center cut.
	pub depth_center_frac: f32,
	/// Bipolar bed-noise amplitude on the bowl Affines (world units).
	pub depth_noise_amp: f32,

	// ── Shore outline (water bowl + wet fill) — higher frequency ───────────
	/// Max bipolar shore indent/expand as a fraction of water radius.
	pub shore_indent_frac: f32,
	/// Boundary-noise frequency for the water shore (angular lobes).
	pub shore_freq: f32,

	// ── Apron / plateau outer outline — lower frequency ────────────────────
	/// Max bipolar apron indent/expand as a fraction of apron width.
	pub apron_indent_frac: f32,
	/// Boundary-noise frequency for the apron/plateau outer (broad lobes).
	pub apron_freq: f32,

	// ── Rim height (add-only above [`Self::rim_lift`]) ──────────────────────
	pub rim_height_amp: f32,
	pub rim_height_freq: f32,

	// ── Fill pad ───────────────────────────────────────────────────────────
	/// Horizontal softmask pad past the bowl, as a fraction of rim width.
	pub rim_bleed_frac: f32,
	/// SDF-relative fade past the fill disc edge.
	pub shore_fade: f32,
}

impl Default for LakeParams {
	fn default() -> Self {
		Self {
			rim_frac: 0.1,
			apron_frac: 0.6,
			mu: 12.0,
			centroid_jitter: 0.12,

			water_scale: 1.0,
			water_scale_min: 0.35,
			water_scale_freq: 0.12,

			rim_width_min: 0.5,
			rim_width_freq: 0.1,

			water_sink: 0.9,
			rim_lift: 1.25,
			terrain_undercut: 2.5,
			shelf_amp: 2.0,
			shelf_sample_count: 6,

			depth: 14.0,
			depth_center_frac: 0.55,
			depth_noise_amp: 2.5,

			shore_indent_frac: 0.25,
			shore_freq: 0.045,

			apron_indent_frac: 0.22,
			apron_freq: 0.02,

			rim_height_amp: 2.75,
			rim_height_freq: 0.016,

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
	/// Rim + apron claim first; water takes a noisy fraction of the leftover so
	/// lakes vary in size without shrinking the apron. `water_u01` / `rim_u01`
	/// ∈ `[0, 1)` should be stable per leaf (same draws in [`Lake::planned_center`] /
	/// [`Lake::from_bounds`]).
	///
	/// Returns `None` when the leaf cannot host a meaningful three-band lake.
	pub fn try_from_short_half(
		short_half: f32,
		params: LakeParams,
		water_u01: f32,
		rim_u01: f32,
	) -> Option<Self> {
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
		let rim_claim = (available * params.rim_frac.clamp(0.05, 0.45))
			.max(available * 0.12)
			.min(available * 0.42);
		let rim_hi = 1.0;
		let rim_lo = params.rim_width_min.clamp(0.2, rim_hi);
		let rim = rim_claim * (rim_lo + (rim_hi - rim_lo) * rim_u01.clamp(0.0, 1.0));
		// Allow large authored aprons (e.g. apron_frac > 1); still leave room for water+rim.
		let apron = (available * params.apron_frac.max(0.05))
			.max(available * 0.14)
			.min(available * 0.72);
		// Leftover uses the *claimed* rim so water undershoot stays independent of rim draw.
		let leftover = (available - rim_claim - apron).min(max_water);
		let size_hi = params.water_scale.clamp(0.05, 1.0);
		let size_lo = params.water_scale_min.clamp(0.05, size_hi);
		let size_frac = size_lo + (size_hi - size_lo) * water_u01.clamp(0.0, 1.0);
		let water = leftover * size_frac;
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

/// Per-leaf water-scale unit sample (shared by centroid planning and stamp build).
fn water_scale_u01(seed: u32, leaf_min: Vec2, params: LakeParams) -> f32 {
	n01_freq(seed, WATER_SCALE_SALT, leaf_min, params.water_scale_freq)
}

/// Per-leaf rim-width unit sample (shared by centroid planning and stamp build).
fn rim_width_u01(seed: u32, leaf_min: Vec2, params: LakeParams) -> f32 {
	n01_freq(seed, RIM_WIDTH_SALT, leaf_min, params.rim_width_freq)
}

/// Mid-rim radius used when surveying surrounding terrain for the shelf anchor.
fn shelf_survey_radius(budget: &LakeBandBudget) -> f32 {
	(budget.water_radius + budget.rim_width * 0.5).max(1.0)
}

/// Median of `samples` (in-place select). Empty → `0.0`.
fn median_f32(samples: &mut [f32]) -> f32 {
	let n = samples.len();
	if n == 0 {
		return 0.0;
	}
	let mid = n / 2;
	let (_, med, _) = samples.select_nth_unstable_by(mid, |a, b| a.total_cmp(b));
	*med
}

/// Sample pre-watershed height around `center` to set the shelf / rim base.
///
/// With `count ≤ 1` (or no sampler), returns the centroid height. Otherwise
/// takes the **median** of `count` evenly spaced samples on a ring at
/// `sample_radius` so a single spike or dip at the center does not own `W`.
pub fn shelf_base_height(
	center: Vec2,
	sample_radius: f32,
	count: u8,
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
) -> f32 {
	let Some(f) = height_at else {
		return 0.0;
	};
	let n = usize::from(count.max(1));
	if n == 1 || sample_radius <= 1e-3 {
		return f(center.x, center.y);
	}
	let mut hs = Vec::with_capacity(n);
	for i in 0..n {
		let ang = (i as f32) * std::f32::consts::TAU / (n as f32);
		let p = center + Vec2::new(ang.cos(), ang.sin()) * sample_radius;
		hs.push(f(p.x, p.y));
	}
	median_f32(&mut hs)
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
	/// Jittered lake centroid used by [`Self::from_bounds`] (for shelf survey).
	pub fn planned_center(bounds: Bounds2, seed: u32, params: LakeParams) -> Vec2 {
		let min = bounds.min;
		let max = bounds.max;
		let cell_c = Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5);
		let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
		let short_half = half.x.min(half.y).max(1.0);
		let u = water_scale_u01(seed, min, params);
		let rim_u = rim_width_u01(seed, min, params);
		let Some(budget) = LakeBandBudget::try_from_short_half(short_half, params, u, rim_u) else {
			return cell_c;
		};
		let shore_amp = budget.water_radius * params.shore_indent_frac.clamp(0.0, 0.45);
		let apron_amp = budget.apron_width * params.apron_indent_frac.clamp(0.0, 0.5);
		let outer = budget.plateau_radius + budget.apron_width + shore_amp.max(apron_amp);
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

		let u = water_scale_u01(seed, min, params);
		let rim_u = rim_width_u01(seed, min, params);
		let Some(budget) = LakeBandBudget::try_from_short_half(short_half, params, u, rim_u) else {
			return empty(Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5));
		};

		let center = Self::planned_center(bounds, seed, params);
		let anchor = min;
		let water_r = budget.water_radius;
		let plateau_r = budget.plateau_radius;
		let rim_w = budget.rim_width;
		let apron_w = budget.apron_width.max(1.0);

		let base_h = shelf_base_height(
			center,
			shelf_survey_radius(&budget),
			params.shelf_sample_count,
			height_at,
		);
		let shelf_anchor = base_h + n11_at(seed, 0x1A7E_50F1, anchor) * params.shelf_amp;
		let water_level = shelf_anchor - params.water_sink.max(0.0);
		let rim_level = shelf_anchor + params.rim_lift.max(0.0);
		let depth = params.depth * (0.65 + 0.7 * n01_at(seed, 0x1A7E_DE07, anchor));
		let bowl_fade = (rim_w * 0.25).max(0.5).min(water_r * 0.2);

		// Modest horizontal pad; terrain_undercut owns shoreline bleed.
		let rim_bleed = rim_w * params.rim_bleed_frac.max(0.0);
		let max_fill = plateau_r + apron_w - 0.5;
		let fill_r = (water_r + rim_bleed).min(max_fill).max(water_r);
		let fill_fade = params.shore_fade.max(1.0);

		let plateau_region = Region2D::Circle(CircleRegion { center, radius: plateau_r });
		let water_region = Region2D::Circle(CircleRegion { center, radius: water_r });
		let fill_region = Region2D::Circle(CircleRegion { center, radius: fill_r });

		// Shore (bowl + fill): higher-freq bipolar indent up to ~¼ water radius.
		let shore_amp = (water_r * params.shore_indent_frac.clamp(0.0, 0.45))
			.min(rim_w * 0.85)
			.max(0.01);
		let shore_freq = params.shore_freq.max(2.2 / water_r.max(1.0)).clamp(1.0e-4, 0.14);
		let shore_noise = RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp);

		// Apron / plateau outer: separate low-freq field (must not inherit shore freq).
		let apron_amp = (apron_w * params.apron_indent_frac.clamp(0.0, 0.5)).max(0.01);
		let apron_noise =
			RegionNoise::from_seed(seed.wrapping_add(6), params.apron_freq.max(1.0e-4), apron_amp);
		let apron_outer = apron_w + apron_amp;

		let rim_height = RegionNoise::from_seed(
			seed.wrapping_add(7),
			params.rim_height_freq.max(1.0e-4),
			params.rim_height_amp.max(0.0),
		);
		let depth_noise_freq = (1.6 / water_r.max(1.0)).clamp(0.04, 0.18);
		let depth_noise = RegionNoise::from_seed(
			seed.wrapping_add(9),
			depth_noise_freq,
			params.depth_noise_amp.max(0.0),
		);

		let plateau = JerseyModulation::Affine(
			RegionAffineModulation::new(plateau_region, 0.0, rim_level, 0.0, apron_outer)
				.with_noise(apron_noise)
				.with_height_noise_add_only(rim_height),
		);

		// Graded bowl: shallow full disc + deeper nested center (depth ↑ toward center).
		let shelf_to_water = (rim_level - water_level).max(0.0);
		let center_frac = params.depth_center_frac.clamp(0.15, 0.75);
		let shallow_cut = depth * (1.0 - center_frac) + shelf_to_water;
		let deep_cut = depth * center_frac;
		let deep_r = (water_r * 0.42).max(MIN_WATER_RADIUS * 0.5).min(water_r * 0.65);
		let deep_region = Region2D::Circle(CircleRegion { center, radius: deep_r });
		let bowl_shallow = JerseyModulation::Affine(
			RegionAffineModulation::new(water_region, 1.0, -shallow_cut, 0.0, bowl_fade)
				.with_noise(shore_noise.clone())
				.with_height_noise(depth_noise.clone()),
		);
		// Center bowl stays geometrically quiet — depth noise only, no shore amp.
		let bowl_deep = JerseyModulation::Affine(
			RegionAffineModulation::new(
				deep_region,
				1.0,
				-deep_cut,
				deep_r * 0.2,
				(deep_r * 0.55).max(bowl_fade),
			)
			.with_height_noise(depth_noise),
		);

		let fill = WaterFill {
			region: fill_region,
			inner_radius: 0.0,
			outer_radius: fill_fade,
			noise: Some(shore_noise),
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
			modulations: vec![plateau, bowl_shallow, bowl_deep],
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
	fn shelf_anchor_uses_ring_median_not_centroid() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let mut params = LakeParams::default();
		params.shelf_amp = 0.0;
		params.shelf_sample_count = 6;
		let center = Lake::planned_center(bounds, 11, params);
		// Flat ring at 40; deep spike only at the centroid.
		let height = |x: f32, z: f32| {
			let d = Vec2::new(x, z).distance(center);
			if d < 2.0 {
				0.0
			} else {
				40.0
			}
		};
		let lake = Lake::from_bounds(bounds, 11, params, Some(&height));
		assert!(!lake.is_empty());
		let expected_w = 40.0 - params.water_sink;
		assert!(
			(lake.water_level - expected_w).abs() < 1e-3,
			"water_level {} should follow ring median 40, not centroid 0 (got vs {expected_w})",
			lake.water_level
		);
		Ok(())
	}

	#[test]
	fn budget_enforces_two_x_body() -> anyhow::Result<()> {
		let short_half = 160.0;
		let budget =
			LakeBandBudget::try_from_short_half(short_half, LakeParams::default(), 1.0, 1.0)
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
	fn water_size_undershoot_varies_radius() -> anyhow::Result<()> {
		let short_half = 160.0;
		let params = LakeParams::default();
		let full = LakeBandBudget::try_from_short_half(short_half, params, 1.0, 1.0).expect("full");
		let mid = LakeBandBudget::try_from_short_half(short_half, params, 0.5, 1.0).expect("mid");
		let small =
			LakeBandBudget::try_from_short_half(short_half, params, 0.0, 1.0).expect("small");
		assert!(full.water_radius > mid.water_radius);
		assert!(mid.water_radius > small.water_radius);
		// Apron stays claimed; size variation is water-only.
		assert!((full.apron_width - small.apron_width).abs() < 1e-4);
		assert!(small.water_radius + 1e-3 >= MIN_WATER_RADIUS);
		Ok(())
	}

	#[test]
	fn rim_width_undershoot_varies_radius() -> anyhow::Result<()> {
		let short_half = 160.0;
		let params = LakeParams::default();
		let wide = LakeBandBudget::try_from_short_half(short_half, params, 1.0, 1.0).expect("wide");
		let narrow =
			LakeBandBudget::try_from_short_half(short_half, params, 1.0, 0.0).expect("narrow");
		assert!(wide.rim_width > narrow.rim_width);
		assert!((wide.water_radius - narrow.water_radius).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn nearby_leaves_draw_different_water_sizes() -> anyhow::Result<()> {
		let params = LakeParams::default();
		let seed = 11u32;
		let mut radii = Vec::new();
		for i in 0..12 {
			let ox = (i as f32) * 37.0;
			let oz = (i as f32) * 29.0;
			let bounds = Bounds2::from_xz(ox, oz, ox + 320.0, oz + 320.0);
			let lake = Lake::from_bounds(bounds, seed, params, Some(&|_, _| 40.0));
			assert!(!lake.is_empty());
			radii.push(lake.water_radius);
		}
		let min_r = radii.iter().cloned().fold(f32::INFINITY, f32::min);
		let max_r = radii.iter().cloned().fold(0.0_f32, f32::max);
		assert!(
			max_r - min_r > 8.0,
			"high-freq size noise should spread water radii across nearby leaves: {radii:?}"
		);
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
		// Sit outward of max shore expand so we stay on the shelf, not the bowl.
		let shore_amp = lake.water_radius * params.shore_indent_frac;
		let mid_r = lake.water_radius + shore_amp + lake.rim_width * 0.55;
		let p = lake.center + Vec2::new(mid_r, 0.0);
		let h = apply_mods(&lake.modulations, base, p.x, p.y);
		let rim_base = lake.water_level + params.rim_lift + params.water_sink;
		assert!(h + 0.25 >= rim_base, "rim {h} should sit at/above base rim {rim_base}");
		assert!(
			h <= rim_base + params.rim_height_amp + 0.75,
			"rim {h} should stay within add-only amp of {rim_base}"
		);
		assert!(h > lake.water_level + 0.25, "rim {h} should sit above water {}", lake.water_level);
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
		let rim_base = lake.water_level + params.rim_lift + params.water_sink;
		let rim_hi = rim_base + params.rim_height_amp;
		let lo = base.min(rim_base);
		let hi = base.max(rim_hi);
		assert!(
			h >= lo - 0.75 && h <= hi + 0.75,
			"apron {h} should sit between identity {base} and rim band [{rim_base}, {rim_hi}]"
		);
		Ok(())
	}

	#[test]
	fn bowl_deeper_at_center() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let lake = Lake::from_bounds(bounds, 11, LakeParams::default(), Some(&|_, _| base));
		assert!(!lake.is_empty());
		let h_c = apply_mods(&lake.modulations, base, lake.center.x, lake.center.y);
		let mid = lake.center + Vec2::new(lake.water_radius * 0.72, 0.0);
		let h_m = apply_mods(&lake.modulations, base, mid.x, mid.y);
		assert!(h_c < h_m - 1.5, "center {h_c} should sit deeper than mid-bowl {h_m}");
		Ok(())
	}

	#[test]
	fn shore_noise_indents_and_expands_water_disc() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let lake = Lake::from_bounds_default(bounds, 11);
		let fill = lake.fills.first().expect("fill");
		// Sample the geometric fill edge (padded past water_r); water_r sits inside the pad.
		let mut wet = 0usize;
		let mut dry = 0usize;
		for i in 0..48 {
			let ang = i as f32 * std::f32::consts::TAU / 48.0;
			let p = lake.center + Vec2::new(ang.cos(), ang.sin()) * lake.fill_radius;
			if softmask_at(fill, p.x, p.y) < 0.5 {
				wet += 1;
			} else {
				dry += 1;
			}
		}
		assert!(
			wet > 0 && dry > 0,
			"bipolar shore noise should both indent and expand at R_fill (wet={wet} dry={dry})"
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
		let outside = lake.center + Vec2::new(lake.plateau_radius + lake.apron_width + 20.0, 0.0);
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
			+ Vec2::new(lake.plateau_radius + lake.apron_width + params.shore_fade + 5.0, 0.0);
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
