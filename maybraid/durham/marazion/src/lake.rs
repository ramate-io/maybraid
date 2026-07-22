//! Lake pocket water — [RFC-127 §3.1.3.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3131-lake).
//!
//! Three-band footprint (center → edge):
//! - **Water** — elliptical bowl depressed below surface `W` (deeper toward centroid)
//! - **Rim** — flat plateau shelf slightly **above** `W` between water edge and plateau edge
//! - **Apron** — soft blend from plateau edge back to identity terrain
//!
//! Axes follow the leaf aspect, inscribed from the jittered centroid (per-edge
//! clearance), with a small noisy rotation. Leaves must still leave room for
//! rim + apron outside the water body.

use crate::fill::{WaterFill, WaterSurface};
use crate::noise::{n01_at, n01_freq, n11_at};
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	EllipseRegion, JerseyModulation, Region2D, RegionAffineModulation, RegionBowlModulation,
	RegionNoise,
};
use procedural_common::Bounds2;

/// Minimum water half-axis (world units); smaller budgets skip the stamp.
const MIN_WATER_RADIUS: f32 = 8.0;

/// Authored `*_freq` knobs are defined at this characteristic water radius.
/// [`scale_noise_freq`] applies a **geometric** (power-law) scale
/// `(ref / radius)^power` — the geometric mean of constant-wavelength and
/// constant-lobe-count when `power = 0.5`.
const NOISE_FREQ_REF_RADIUS: f32 = 80.0;

/// Salt for per-leaf water-radius undershoot.
const WATER_SCALE_SALT: u32 = 0x1A7E_512E;
/// Salt for per-leaf rim-width undershoot.
const RIM_WIDTH_SALT: u32 = 0x1A7E_71B7;
/// Salt for aspect blend (circle ↔ full leaf aspect).
const ASPECT_SALT: u32 = 0x1A7E_A5EC;
/// Salt for ellipse rotation.
const ROTATION_SALT: u32 = 0x1A7E_207A;

/// Authoring knobs for a Marazion lake stamp.
///
/// Grouped by what they control: footprint budget → per-leaf scales → vertical
/// levels → bowl depth → shore outline → apron outline → rim height.
#[derive(Debug, Clone, Copy)]
pub struct LakeParams {
	// ── Footprint budget ───────────────────────────────────────────────────
	/// Rim shelf width as a fraction of the leaf clearance budget.
	pub rim_frac: f32,
	/// Apron (blend-to-identity) width as a fraction of the leaf clearance budget.
	pub apron_frac: f32,
	/// Inward margin μ (world units) from cell boundary to the outer apron.
	pub mu: f32,
	/// Max centroid offset as a fraction of the smaller cell half-extent.
	pub centroid_jitter: f32,
	/// How far toward full leaf aspect (`1`) vs circular (`0`) on **large** leaves.
	pub aspect_strength: f32,
	/// Fraction of [`Self::aspect_strength`] applied on small leaves (≤
	/// [`Self::aspect_scale_ref`]). Ramps geometrically toward full strength.
	pub aspect_small: f32,
	/// Floor on the aspect draw at large scale (mild oval even when the draw is 0).
	/// Small leaves use ~0; climbs with size via [`Self::aspect_scale_ref`].
	pub aspect_floor: f32,
	/// Short leaf half-extent (world units) where anti-circle is still “small”
	/// ([`Self::aspect_small`]). Above this, strength/floor ramp up geometrically.
	pub aspect_scale_ref: f32,
	/// Max fraction of long-axis clearance the water body may claim (after rim/apron).
	pub long_axis_frac: f32,
	/// Max |rotation| of the ellipse (radians).
	pub rotation_amp: f32,
	/// Power for noise frequency scaling: `f ∝ (ref / radius)^power`.
	/// `0.5` ≈ geometric mean of constant Hz and constant lobe count; `1.0` = linear.
	pub noise_freq_power: f32,

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
	/// How far the wet-column gate bites into terrain (`h − undercut`).
	/// Wet columns are half-spaces below \(W\); undercut only decides which
	/// columns count as wet (shoreline under raised rims).
	pub terrain_undercut: f32,
	/// Per-leaf jitter on the shelf anchor height that sets `W` and rim base.
	pub shelf_amp: f32,
	/// How many pre-watershed heights to sample on a ring around the centroid
	/// (at mid-rim radius) when setting the shelf anchor. `1` = centroid only.
	pub shelf_sample_count: u8,

	// ── Bowl depth ─────────────────────────────────────────────────────────
	/// Bowl depth scale at the centroid (world units below `W`).
	pub depth: f32,
	/// Exponent on `(1 - r)` for the radial depth falloff.
	pub depth_falloff_power: f32,
	/// Bipolar bed-noise amplitude (world units); may raise bed above `W`.
	pub depth_noise_amp: f32,
	/// Bed-noise frequency at [`NOISE_FREQ_REF_RADIUS`] (scaled in
	/// [`Lake::from_bounds`] by `(ref / short_water)^noise_freq_power`).
	pub depth_noise_freq: f32,
	/// Extra headroom above the rim shelf for island / peninsula peaks.
	pub island_lift: f32,

	// ── Shore outline (water bowl + wet fill) — higher frequency ───────────
	/// Max bipolar shore indent/expand as a fraction of the short water axis.
	pub shore_indent_frac: f32,
	/// Shore boundary frequency at [`NOISE_FREQ_REF_RADIUS`] (scaled geometrically
	/// in [`Lake::from_bounds`]).
	pub shore_freq: f32,

	// ── Apron / plateau outer outline — lower frequency ────────────────────
	/// Max bipolar apron indent/expand as a fraction of apron width.
	pub apron_indent_frac: f32,
	/// Apron boundary frequency at [`NOISE_FREQ_REF_RADIUS`] (scaled geometrically
	/// in [`Lake::from_bounds`]).
	pub apron_freq: f32,

	// ── Rim height (add-only above [`Self::rim_lift`]) ──────────────────────
	pub rim_height_amp: f32,
	/// Rim height-noise frequency at [`NOISE_FREQ_REF_RADIUS`] (scaled like shore).
	pub rim_height_freq: f32,

	// ── Fill pad ───────────────────────────────────────────────────────────
	/// Horizontal softmask pad past the bowl, as a fraction of rim width.
	pub rim_bleed_frac: f32,
	/// SDF-relative fade past the fill edge.
	pub shore_fade: f32,
}

impl Default for LakeParams {
	fn default() -> Self {
		Self {
			rim_frac: 0.1,
			apron_frac: 0.6,
			mu: 12.0,
			centroid_jitter: 0.12,
			aspect_strength: 0.95,
			aspect_small: 0.22,
			aspect_floor: 0.55,
			aspect_scale_ref: 280.0,
			long_axis_frac: 0.78,
			rotation_amp: 0.55,
			noise_freq_power: 0.5,

			water_scale: 1.0,
			water_scale_min: 0.35,
			water_scale_freq: 0.12,

			rim_width_min: 0.5,
			rim_width_freq: 0.1,

			water_sink: 0.9,
			rim_lift: 1.25,
			terrain_undercut: 1.75,
			shelf_amp: 2.0,
			shelf_sample_count: 6,

			depth: 14.0,
			depth_falloff_power: 1.35,
			depth_noise_amp: 8.0,
			depth_noise_freq: 0.016,
			island_lift: 5.5,

			shore_indent_frac: 0.18,
			// Authored at ref; √ scale + min(1). Slightly below the old linear-era
			// knobs so high-pass effective Hz stay near the look that worked, while
			// sub-ref ponds no longer inherit an amplified harshness.
			shore_freq: 0.022,

			apron_indent_frac: 0.22,
			apron_freq: 0.011,

			rim_height_amp: 2.75,
			rim_height_freq: 0.009,

			rim_bleed_frac: 0.35,
			shore_fade: 2.0,
		}
	}
}

/// Elliptical band budget derived from per-axis clearance at the lake centroid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LakeBandBudget {
	/// Water half-axes in the ellipse local frame.
	pub water_radii: Vec2,
	pub rotation: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	/// Plateau half-axes (`water + rim`) in the local frame.
	pub plateau_radii: Vec2,
	pub mu: f32,
}

impl LakeBandBudget {
	/// Characteristic (short) water half-axis.
	pub fn water_radius(&self) -> f32 {
		self.water_radii.min_element()
	}

	/// Characteristic (short) plateau half-axis.
	pub fn plateau_radius(&self) -> f32 {
		self.plateau_radii.min_element()
	}

	/// Budget water / rim / apron from inscribed clearance at `center`.
	///
	/// Rim + apron claim first (isotropic widths); water takes a noisy fraction
	/// of the leftover on each axis, blended between circular and full leaf
	/// aspect. A small rotation is applied and axes are rescaled so the rotated
	/// ellipse AABB still fits. `water_u01` / `rim_u01` / `aspect_u01` /
	/// `rotation_u11` should be stable per leaf.
	///
	/// Returns `None` when the leaf cannot host a meaningful three-band lake.
	pub fn try_inscribed(
		bounds: Bounds2,
		center: Vec2,
		params: LakeParams,
		water_u01: f32,
		rim_u01: f32,
		aspect_u01: f32,
		rotation_u11: f32,
	) -> Option<Self> {
		let min = bounds.min;
		let max = bounds.max;
		let room = Vec2::new(
			(center.x - min.x).min(max.x - center.x).max(0.0),
			(center.y - min.y).min(max.y - center.y).max(0.0),
		);
		let short_room = room.min_element();
		if short_room < MIN_WATER_RADIUS * 2.0 {
			return None;
		}
		let mu = params.mu.min(short_room * 0.2).max(0.0);
		let available = (room - Vec2::splat(mu)).max(Vec2::ZERO);
		let short_avail = available.min_element();
		if short_avail < MIN_WATER_RADIUS * 2.0 {
			return None;
		}

		// Enforce leaf clearance ≥ 2 · body diameter on the short axis.
		let max_water_short = (short_room * 0.5).min(short_avail * 0.45);
		let rim_claim = (short_avail * params.rim_frac.clamp(0.05, 0.45))
			.max(short_avail * 0.12)
			.min(short_avail * 0.42);
		let rim_hi = 1.0;
		let rim_lo = params.rim_width_min.clamp(0.2, rim_hi);
		let rim = rim_claim * (rim_lo + (rim_hi - rim_lo) * rim_u01.clamp(0.0, 1.0));
		let apron = (short_avail * params.apron_frac.max(0.05))
			.max(short_avail * 0.14)
			.min(short_avail * 0.72);

		// Per-axis leftover after isotropic rim+apron claim. Cap the short axis
		// so leaf ≥ 2× body; the long axis may use more of its clearance.
		let leftover = Vec2::new(
			(available.x - rim_claim - apron).max(0.0),
			(available.y - rim_claim - apron).max(0.0),
		);
		let long_frac = params.long_axis_frac.clamp(0.45, 0.95);
		let leftover = Vec2::new(
			if available.x <= available.y {
				leftover.x.min(max_water_short)
			} else {
				leftover.x.min(available.x * long_frac)
			},
			if available.y <= available.x {
				leftover.y.min(max_water_short)
			} else {
				leftover.y.min(available.y * long_frac)
			},
		);

		let size_hi = params.water_scale.clamp(0.05, 1.0);
		let size_lo = params.water_scale_min.clamp(0.05, size_hi);
		let size_frac = size_lo + (size_hi - size_lo) * water_u01.clamp(0.0, 1.0);

		let circ = leftover.min_element() * size_frac;
		let full = leftover * size_frac;
		let aspect = aspect_blend(params, short_room, aspect_u01);
		let mut water = Vec2::new(
			circ + (full.x - circ) * aspect,
			circ + (full.y - circ) * aspect,
		);
		if water.min_element() < MIN_WATER_RADIUS {
			return None;
		}
		// Re-check 2× body vs short leaf room.
		if 2.0 * short_room < 2.0 * (2.0 * water.min_element()) * 0.99 {
			return None;
		}

		let rotation = rotation_u11.clamp(-1.0, 1.0) * params.rotation_amp.max(0.0);
		// Shrink so rotated water+rim+apron AABB fits inscribed clearance.
		let band = rim + apron;
		let fit_radii = water + Vec2::splat(band);
		let (s, c) = rotation.sin_cos();
		let aabb = Vec2::new(
			((fit_radii.x * c).abs().powi(2) + (fit_radii.y * s).abs().powi(2)).sqrt(),
			((fit_radii.x * s).abs().powi(2) + (fit_radii.y * c).abs().powi(2)).sqrt(),
		);
		let scale = (available / aabb.max(Vec2::splat(1e-3)))
			.min_element()
			.clamp(0.0, 1.0);
		water *= scale;
		if water.min_element() < MIN_WATER_RADIUS {
			return None;
		}

		Some(Self {
			water_radii: water,
			rotation,
			rim_width: rim,
			apron_width: apron,
			plateau_radii: water + Vec2::splat(rim),
			mu,
		})
	}

	/// Axis-aligned helper used by unit tests (centroid = leaf center, no rotation).
	pub fn try_from_short_half(
		short_half: f32,
		params: LakeParams,
		water_u01: f32,
		rim_u01: f32,
	) -> Option<Self> {
		let s = short_half.max(0.0);
		let bounds = Bounds2::from_xz(-s, -s, s, s);
		Self::try_inscribed(bounds, Vec2::ZERO, params, water_u01, rim_u01, 0.0, 0.0)
	}
}

/// Scale an authored frequency from [`NOISE_FREQ_REF_RADIUS`] to `radius`.
///
/// ```text
/// f = f_ref * (ref / radius)^power
/// ```
///
/// `power = 0.5` is the geometric mean of constant wavelength (`^0`) and
/// constant lobe count (`^1`). Linear `power = 1` over-harshens small lakes
/// and over-smooths the path between bands; √ tracks perceived roughness better.
///
/// Sub-ref radii still clamp the scale to ≤ 1 so ponds never exceed the
/// authored reference roughness.
fn scale_noise_freq(authored_at_ref: f32, radius: f32, power: f32) -> f32 {
	let r = radius.max(1.0);
	let ratio = NOISE_FREQ_REF_RADIUS / r;
	let scale = ratio.powf(power.clamp(0.15, 2.0)).min(1.0);
	(authored_at_ref.max(0.0) * scale).clamp(1.0e-4, 0.14)
}

/// Aspect blend ∈ `[0, 1]` — weak on small leaves, strong on large.
///
/// ```text
/// t = 1 - 1/sqrt(max(short_room/ref, 1))   // 0 at ref, →1 as size grows
/// strength = aspect_strength * lerp(aspect_small, 1, t)
/// floor    = aspect_floor * t
/// blend    = strength * lerp(floor, 1, aspect_u01)
/// ```
fn aspect_blend(params: LakeParams, short_room: f32, aspect_u01: f32) -> f32 {
	let u = aspect_u01.clamp(0.0, 1.0);
	let ratio = (short_room / params.aspect_scale_ref.max(1.0)).max(1.0e-3);
	// Geometric size factor: flat (0) while small_room ≤ ref, then √ ramp.
	let t = (1.0 - 1.0 / ratio.max(1.0).sqrt()).clamp(0.0, 1.0);
	let small = params.aspect_small.clamp(0.0, 1.0);
	let strength = params.aspect_strength.clamp(0.0, 1.0) * (small + (1.0 - small) * t);
	let floor = (params.aspect_floor.clamp(0.0, 0.95) * t).clamp(0.0, 0.9);
	(strength * (floor + (1.0 - floor) * u)).clamp(0.0, 1.0)
}

/// Per-leaf water-scale unit sample (shared by centroid planning and stamp build).
fn water_scale_u01(seed: u32, leaf_min: Vec2, params: LakeParams) -> f32 {
	n01_freq(seed, WATER_SCALE_SALT, leaf_min, params.water_scale_freq)
}

/// Per-leaf rim-width unit sample (shared by centroid planning and stamp build).
fn rim_width_u01(seed: u32, leaf_min: Vec2, params: LakeParams) -> f32 {
	n01_freq(seed, RIM_WIDTH_SALT, leaf_min, params.rim_width_freq)
}

fn aspect_u01(seed: u32, leaf_min: Vec2) -> f32 {
	n01_at(seed, ASPECT_SALT, leaf_min)
}

fn rotation_u11(seed: u32, leaf_min: Vec2) -> f32 {
	n11_at(seed, ROTATION_SALT, leaf_min)
}

fn ellipse_region(center: Vec2, radii: Vec2, rotation: f32) -> Region2D {
	Region2D::Ellipse(EllipseRegion {
		center,
		radii: radii.max(Vec2::splat(1e-3)),
		rotation,
	})
}

/// Mid-rim characteristic radius used when surveying surrounding terrain.
fn shelf_survey_radius(budget: &LakeBandBudget) -> f32 {
	(budget.water_radius() + budget.rim_width * 0.5).max(1.0)
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
	/// Water half-axes in the ellipse local frame.
	pub water_radii: Vec2,
	pub rotation: f32,
	/// Characteristic (short) water half-axis.
	pub water_radius: f32,
	/// Characteristic (short) plateau half-axis.
	pub plateau_radius: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	/// Characteristic fill half-axis (≥ [`Self::water_radius`]).
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
		// Conservative outer pad from a centered circular budget so the clamp
		// box stays valid before the inscribed elliptical budget is known.
		let u = water_scale_u01(seed, min, params);
		let rim_u = rim_width_u01(seed, min, params);
		let Some(probe) = LakeBandBudget::try_from_short_half(short_half, params, u, rim_u) else {
			return cell_c;
		};
		let shore_amp = probe.water_radius() * params.shore_indent_frac.clamp(0.0, 0.45);
		let apron_amp = probe.apron_width * params.apron_indent_frac.clamp(0.0, 0.5);
		let outer = probe.plateau_radius() + probe.apron_width + shore_amp.max(apron_amp);
		let lo = min + Vec2::splat(outer.max(probe.mu));
		let hi = max - Vec2::splat(outer.max(probe.mu));
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
		let empty = |center: Vec2| Self {
			bounds,
			seed,
			center,
			water_radii: Vec2::ZERO,
			rotation: 0.0,
			water_radius: 0.0,
			plateau_radius: 0.0,
			rim_width: 0.0,
			apron_width: 0.0,
			fill_radius: 0.0,
			water_level: 0.0,
			modulations: Vec::new(),
			fills: Vec::new(),
		};

		let center = Self::planned_center(bounds, seed, params);
		let u = water_scale_u01(seed, min, params);
		let rim_u = rim_width_u01(seed, min, params);
		let asp = aspect_u01(seed, min);
		let rot = rotation_u11(seed, min);
		let Some(budget) =
			LakeBandBudget::try_inscribed(bounds, center, params, u, rim_u, asp, rot)
		else {
			return empty(Vec2::new(
				(bounds.min.x + bounds.max.x) * 0.5,
				(bounds.min.y + bounds.max.y) * 0.5,
			));
		};

		let anchor = min;
		let water_r = budget.water_radii;
		let plateau_r = budget.plateau_radii;
		let rim_w = budget.rim_width;
		let apron_w = budget.apron_width.max(1.0);
		let rotation = budget.rotation;
		let short_water = budget.water_radius();

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
		let bowl_fade = (rim_w * 0.25).max(0.5).min(short_water * 0.2);

		let rim_bleed = rim_w * params.rim_bleed_frac.max(0.0);
		let fill_r = water_r + Vec2::splat(rim_bleed);
		// Keep fill inside plateau+apron on each local axis.
		let max_fill = plateau_r + Vec2::splat(apron_w - 0.5);
		let fill_r = Vec2::new(fill_r.x.min(max_fill.x), fill_r.y.min(max_fill.y)).max(water_r);
		let fill_fade = params.shore_fade.max(1.0);

		let plateau_region = ellipse_region(center, plateau_r, rotation);
		let water_region = ellipse_region(center, water_r, rotation);
		let fill_region = ellipse_region(center, fill_r, rotation);

		// ── Size-scaled noise (authored freqs @ NOISE_FREQ_REF_RADIUS) ─────
		// Shore / apron / rim / bed all use the short water axis so relative
		// lobe counts stay consistent from low-pass ponds to high-pass lakes.
		let shore_amp = (short_water * params.shore_indent_frac.clamp(0.0, 0.45))
			.min(rim_w * 0.85)
			.max(0.01);
		let shore_freq =
			scale_noise_freq(params.shore_freq, short_water, params.noise_freq_power);
		let shore_noise = RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp);

		let apron_amp = (apron_w * params.apron_indent_frac.clamp(0.0, 0.5)).max(0.01);
		let apron_freq =
			scale_noise_freq(params.apron_freq, short_water, params.noise_freq_power);
		let apron_noise = RegionNoise::from_seed(seed.wrapping_add(6), apron_freq, apron_amp);
		let apron_outer = apron_w + apron_amp;

		let rim_height_freq =
			scale_noise_freq(params.rim_height_freq, short_water, params.noise_freq_power);
		let rim_height = RegionNoise::from_seed(
			seed.wrapping_add(7),
			rim_height_freq,
			params.rim_height_amp.max(0.0),
		);
		let depth_noise_freq =
			scale_noise_freq(params.depth_noise_freq, short_water, params.noise_freq_power);
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

		// Single radial bowl: deeper toward centroid; bipolar bed noise may
		// emerge above W (islands / peninsulas). Ceiling sits above the rim
		// so peaks can read clearly instead of flattening into the shelf.
		let undercut = params.terrain_undercut.max(0.0);
		let bed_ceiling = (rim_level + params.island_lift.max(0.0))
			.max(water_level + undercut + params.depth_noise_amp.max(0.0) * 0.85);
		let center_bed = water_level - depth;
		let shore_bed = water_level;
		let bowl = JerseyModulation::Bowl(
			RegionBowlModulation::new(
				water_region,
				center_bed,
				shore_bed,
				bed_ceiling,
				params.depth_falloff_power,
				bowl_fade,
			)
			.with_boundary_noise(shore_noise.clone())
			.with_bed_noise(depth_noise),
		);

		let fill = WaterFill {
			region: fill_region,
			inner_radius: 0.0,
			outer_radius: fill_fade,
			noise: Some(shore_noise),
			surface: WaterSurface::Flat { level: water_level },
			terrain_undercut: undercut,
		};

		Self {
			bounds,
			seed,
			center,
			water_radii: water_r,
			rotation,
			water_radius: short_water,
			plateau_radius: budget.plateau_radius(),
			rim_width: rim_w,
			apron_width: budget.apron_width,
			fill_radius: fill_r.min_element(),
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
		let body_diameter = 2.0 * budget.water_radius();
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
		assert!(full.water_radius() > mid.water_radius());
		assert!(mid.water_radius() > small.water_radius());
		// Apron stays claimed; size variation is water-only.
		assert!((full.apron_width - small.apron_width).abs() < 1e-4);
		assert!(small.water_radius() + 1e-3 >= MIN_WATER_RADIUS);
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
		assert!((wide.water_radius() - narrow.water_radius()).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn elongated_leaf_gets_aspect_tied_ellipse() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 200.0, 480.0);
		let mut params = LakeParams::default();
		params.aspect_strength = 1.0;
		params.rotation_amp = 0.0;
		let center = Vec2::new(100.0, 240.0);
		let budget = LakeBandBudget::try_inscribed(bounds, center, params, 1.0, 1.0, 1.0, 0.0)
			.expect("budget");
		assert!(
			budget.water_radii.y > budget.water_radii.x * 1.25,
			"tall leaf should elongate water_radii: {:?}",
			budget.water_radii
		);
		Ok(())
	}

	#[test]
	fn noise_freq_scales_geometrically_and_caps_small() -> anyhow::Result<()> {
		let power = 0.5;
		let f_ref = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS, power);
		let f_small = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS * 0.5, power);
		let f_large = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS * 10.0, power);
		assert!((f_ref - 0.04).abs() < 1e-5);
		assert!(
			(f_small - f_ref).abs() < 1e-5,
			"sub-ref lakes must not exceed authored freq (linear over-harshened them)"
		);
		let expected_large = 0.04 * (0.1_f32).sqrt();
		assert!(
			(f_large - expected_large).abs() < 1e-4,
			"10× radius with √ power: got {f_large}, want ~{expected_large}"
		);
		let f_linear = scale_noise_freq(0.04, NOISE_FREQ_REF_RADIUS * 10.0, 1.0);
		assert!(
			f_large > f_linear * 1.5,
			"√ scale should quiet large lakes less aggressively than linear"
		);
		Ok(())
	}

	#[test]
	fn aspect_blend_strengthens_on_large_leaves() -> anyhow::Result<()> {
		let params = LakeParams::default();
		let small = aspect_blend(params, params.aspect_scale_ref * 0.5, 0.5);
		let large = aspect_blend(params, params.aspect_scale_ref * 4.0, 0.5);
		assert!(
			large > small + 0.15,
			"large-leaf aspect should exceed small-leaf: small={small} large={large}"
		);
		// Small leaves stay near aspect_small * strength * u (no floor).
		let small_hi = aspect_blend(params, params.aspect_scale_ref * 0.5, 1.0);
		assert!(
			small_hi < params.aspect_strength * 0.45,
			"small-leaf max aspect should stay mild, got {small_hi}"
		);
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
		let mut params = LakeParams::default();
		params.depth_noise_amp = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base));
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
		let mut params = LakeParams::default();
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
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
		let mut params = LakeParams::default();
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
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
		let mut params = LakeParams::default();
		params.depth_noise_amp = 0.0;
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base));
		assert!(!lake.is_empty());
		let h_c = apply_mods(&lake.modulations, base, lake.center.x, lake.center.y);
		let mid = lake.center + Vec2::new(lake.water_radius * 0.72, 0.0);
		let h_m = apply_mods(&lake.modulations, base, mid.x, mid.y);
		assert!(h_c < h_m - 1.5, "center {h_c} should sit deeper than mid-bowl {h_m}");
		Ok(())
	}

	#[test]
	fn bed_noise_can_rise_above_water() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let mut params = LakeParams::default();
		params.depth_noise_amp = 10.0;
		params.depth_noise_freq = 0.04;
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
		params.aspect_floor = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, Some(&|_, _| base));
		let mut raised = false;
		// Near-shore rings: base bed is closer to W so amp can crest above it.
		for &frac in &[0.55_f32, 0.72, 0.85] {
			for i in 0..48 {
				let ang = i as f32 * std::f32::consts::TAU / 48.0;
				let p = lake.center + Vec2::new(ang.cos(), ang.sin()) * (lake.water_radius * frac);
				let h = apply_mods(&lake.modulations, base, p.x, p.y);
				if h > lake.water_level + 0.25 {
					raised = true;
					break;
				}
			}
			if raised {
				break;
			}
		}
		assert!(raised, "bed noise should lift some near-shore samples above W");
		Ok(())
	}

	#[test]
	fn shore_noise_indents_and_expands_water_disc() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let mut params = LakeParams::default();
		params.rotation_amp = 0.0;
		params.aspect_strength = 0.0;
		let lake = Lake::from_bounds(bounds, 11, params, None);
		let fill = lake.fills.first().expect("fill");
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
		assert!(lake.fill_radius + 1e-3 >= lake.water_radius);
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
