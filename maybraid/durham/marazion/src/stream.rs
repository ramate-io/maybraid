//! Stream pocket water — [RFC-127 §3.1.3.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3132-stream).
//!
//! Distance-to-polyline bands (centerline → edge):
//! - **Thalweg** — deepest cut along the path
//! - **Wet channel** — floor near graded surface \(W(s)\)
//! - **Skirt / rim** — bank lift slightly above \(W(s)\)
//!
//! Fill / channel grade is piecewise along path nodes (segment lerp + pitch blend).

use crate::fill::{WaterFill, WaterSurface};
use crate::noise::{n01_at, n01_freq, scale_noise_freq};
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	DownhillPair, HysteresisSpine, JerseyModulation, PolylineRegion, RegionAffineModulation,
	Region2D, RegionNoise, RegionPolylineGradingModulation,
};
use procedural_common::Bounds2;

/// Minimum channel half-width (world units); smaller budgets skip the stamp.
const MIN_HALF_WIDTH: f32 = 3.0;
/// Minimum path length (world units).
const MIN_PATH_LEN: f32 = 24.0;

const WIDTH_SCALE_SALT: u32 = 0x57EA_512E;
const ENDPOINT_A_SALT: u32 = 0x57EA_E001;
const ENDPOINT_B_SALT: u32 = 0x57EA_E002;
/// Salt for per-leaf rim height amplitude draw.
const RIM_HEIGHT_AMP_SALT: u32 = 0x57EA_A17A;
/// Salt for per-leaf rim height frequency draw.
const RIM_HEIGHT_FREQ_SALT: u32 = 0x57EA_F7E9;
/// Salt for per-leaf apron indent amplitude draw.
const APRON_AMP_SALT: u32 = 0x57EA_A70A;
/// Salt for per-leaf apron frequency draw.
const APRON_FREQ_SALT: u32 = 0x57EA_AF7E;

/// Authoring knobs for a Marazion stream stamp.
#[derive(Debug, Clone, Copy)]
pub struct StreamParams {
	/// Channel half-width as a fraction of the leaf short half-extent.
	pub half_width_frac: f32,
	/// Min half-width scale (`1.0` = full claimed width).
	pub half_width_scale_min: f32,
	/// Max half-width scale.
	pub half_width_scale: f32,
	pub half_width_freq: f32,

	/// Thalweg half-width as a fraction of channel half-width.
	pub thalweg_ratio: f32,
	/// Skirt / bank width beyond the wet channel (world units fraction of half-width).
	pub skirt_extra_frac: f32,
	/// Soft apron past the skirt (blend to identity), as a fraction of half-width.
	pub apron_frac: f32,
	/// Inward margin μ from cell boundary when placing endpoints.
	pub mu: f32,

	/// How far below endpoint terrain the water surface sits.
	pub water_sink: f32,
	/// Bank lift above the graded surface.
	pub rim_lift: f32,
	/// Minimum forced drop between consecutive nodes when a segment is flat/uphill.
	pub min_drop: f32,
	/// Path-distance radius for inbound/outbound pitch blending at vertices.
	/// When `0`, uses ~half a hysteresis step (from [`Self::spine`]).
	pub node_blend: f32,

	/// Thalweg cut depth (world units).
	pub depth: f32,
	pub depth_noise_amp: f32,

	/// Shore / bank outline amplitude as a fraction of half-width (terrain mods only).
	pub shore_indent_frac: f32,
	pub shore_freq: f32,

	/// Power for noise frequency scaling: `f ∝ (ref / radius)^power`.
	/// `0.5` ≈ geometric mean of constant Hz and constant lobe count; `1.0` = linear.
	pub noise_freq_power: f32,

	// ── Apron / skirt outer outline — lower frequency ──────────────────────
	/// Per-leaf apron boundary indent as a fraction of apron-band width (low).
	pub apron_indent_frac_min: f32,
	/// Per-leaf apron boundary indent as a fraction of apron-band width (high).
	pub apron_indent_frac_max: f32,
	/// Per-leaf apron boundary frequency low (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub apron_freq_min: f32,
	/// Per-leaf apron boundary frequency high (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub apron_freq_max: f32,

	// ── Rim height (add-only above [`Self::rim_lift`]) ──────────────────────
	/// Per-leaf rim height-noise amplitude low (world units); [`Stream::from_bounds`]
	/// draws uniformly in `[min, max]`.
	pub rim_height_amp_min: f32,
	/// Per-leaf rim height-noise amplitude high (world units).
	pub rim_height_amp_max: f32,
	/// Per-leaf rim height-noise frequency low (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub rim_height_freq_min: f32,
	/// Per-leaf rim height-noise frequency high (at [`crate::noise::NOISE_FREQ_REF_RADIUS`]).
	pub rim_height_freq_max: f32,

	/// Softmask fade past the fill support edge (world units). Stacks on
	/// [`Self::fill_half_width_scale`] so the wet ribbon is liberal vs the carve.
	pub shore_fade: f32,

	/// How far below the water surface \(W\) the channel floor grade sits.
	///
	/// Keeps the carved bed under \(W\) so the wet-column gate stays open;
	/// fill itself is a half-space below \(W\) (see [`WaterFill`]).
	pub channel_freeboard: f32,

	/// Fill support half-width as a multiple of the carved channel half-width.
	/// Prefer `> 1` so MC gets a wider wet ribbon than the visible cut.
	pub fill_half_width_scale: f32,

	/// Extra fill undercut for the wet-column gate under banks / noise.
	pub fill_undercut: f32,

	/// Hysteresis spine walk (step / snap) — uses Jersey defaults when left at default.
	pub spine: HysteresisSpine,
}

impl Default for StreamParams {
	fn default() -> Self {
		Self {
			half_width_frac: 0.08,
			half_width_scale_min: 0.55,
			half_width_scale: 1.0,
			half_width_freq: 0.1,

			thalweg_ratio: 0.35,
			skirt_extra_frac: 0.85,
			apron_frac: 1.4,
			mu: 10.0,

			water_sink: 0.7,
			rim_lift: 1.1,
			min_drop: 0.75,
			node_blend: 0.0,

			depth: 8.0,
			depth_noise_amp: 1.5,

			shore_indent_frac: 0.2,
			shore_freq: 0.04,

			noise_freq_power: 0.5,

			apron_indent_frac_min: 0.12,
			apron_indent_frac_max: 0.40,
			apron_freq_min: 0.005,
			apron_freq_max: 0.012,

			rim_height_amp_min: 15.0,
			rim_height_amp_max: 120.0,
			rim_height_freq_min: 0.005,
			rim_height_freq_max: 0.012,

			shore_fade: 4.0,
			channel_freeboard: 2.0,
			fill_half_width_scale: 1.35,
			fill_undercut: 2.0,

			spine: HysteresisSpine::default(),
		}
	}
}

/// Band widths derived from a leaf short half-extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamBandBudget {
	pub half_width: f32,
	pub thalweg_half: f32,
	pub skirt_half: f32,
	pub apron_half: f32,
	pub mu: f32,
}

impl StreamBandBudget {
	/// Returns `None` when the leaf cannot host a meaningful stream corridor.
	pub fn try_from_short_half(short_half: f32, params: StreamParams, width_u01: f32) -> Option<Self> {
		let s = short_half.max(0.0);
		let mu = params.mu.min(s * 0.25).max(0.0);
		let available = (s - mu).max(0.0);
		if available < MIN_HALF_WIDTH * 3.0 {
			return None;
		}
		let claim = (available * params.half_width_frac.clamp(0.03, 0.22))
			.max(MIN_HALF_WIDTH)
			.min(available * 0.28);
		let hi = params.half_width_scale.clamp(0.2, 1.0);
		let lo = params.half_width_scale_min.clamp(0.2, hi);
		let half = claim * (lo + (hi - lo) * width_u01.clamp(0.0, 1.0));
		if half < MIN_HALF_WIDTH {
			return None;
		}
		let thalweg = (half * params.thalweg_ratio.clamp(0.15, 0.7)).max(1.0);
		let skirt = half * (1.0 + params.skirt_extra_frac.max(0.2));
		let apron = skirt + half * params.apron_frac.max(0.4);
		Some(Self {
			half_width: half,
			thalweg_half: thalweg,
			skirt_half: skirt,
			apron_half: apron,
			mu,
		})
	}
}

fn width_scale_u01(seed: u32, leaf_min: Vec2, params: StreamParams) -> f32 {
	n01_freq(seed, WIDTH_SCALE_SALT, leaf_min, params.half_width_freq)
}

fn sample_endpoint(seed: u32, salt: u32, lo: Vec2, hi: Vec2) -> Vec2 {
	let ux = n01_at(seed, salt, lo);
	let uz = n01_at(seed, salt.wrapping_add(1), lo);
	Vec2::new(
		lo.x + (hi.x - lo.x) * ux,
		lo.y + (hi.y - lo.y) * uz,
	)
}

/// Sample per-node water elevations along a path (pre-watershed heights − sink).
///
/// Local segment pitches follow the samples; uphill chords are clamped so each
/// node is non-increasing vs its upstream neighbor. If the whole reach is nearly
/// flat, the toe is pulled down by `min_drop`.
fn node_water_levels(
	path: &[Vec2],
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
	sink: f32,
	min_drop: f32,
) -> Vec<f32> {
	let sink = sink.max(0.0);
	let min_drop = min_drop.max(0.0);
	let mut levels: Vec<f32> = path
		.iter()
		.map(|p| height_at.map(|f| f(p.x, p.y)).unwrap_or(0.0) - sink)
		.collect();
	for i in 1..levels.len() {
		levels[i] = levels[i].min(levels[i - 1]);
	}
	if levels.len() >= 2 {
		let head = levels[0];
		let last = levels.len() - 1;
		if head - levels[last] < min_drop {
			levels[last] = head - min_drop;
			for i in (1..last).rev() {
				levels[i] = levels[i]
					.min(levels[i - 1])
					.max(levels[last]);
			}
		}
	}
	levels
}

fn bank_levels(water_levels: &[f32], rim_lift: f32) -> Vec<f32> {
	let lift = rim_lift.max(0.0);
	water_levels.iter().map(|w| w + lift).collect()
}

/// Channel floor grade: water surface levels minus freeboard (strictly below \(W\)).
fn bed_levels(water_levels: &[f32], freeboard: f32) -> Vec<f32> {
	let fb = freeboard.max(0.25);
	water_levels.iter().map(|w| w - fb).collect()
}

/// Stream stamp products for one pocket-water leaf.
#[derive(Debug, Clone)]
pub struct Stream {
	pub bounds: Bounds2,
	pub seed: u32,
	pub path: Vec<Vec2>,
	/// Per-vertex water surface elevations along [`Self::path`].
	pub levels: Vec<f32>,
	pub half_width: f32,
	pub thalweg_half: f32,
	pub skirt_half: f32,
	pub head_water: f32,
	pub toe_water: f32,
	pub modulations: Vec<JerseyModulation>,
	pub fills: Vec<WaterFill>,
}

impl Stream {
	/// Build a graded stream, or an empty stamp when the leaf / path is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: StreamParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let min = bounds.min;
		let max = bounds.max;
		let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
		let short_half = half.x.min(half.y).max(1.0);
		let empty = Self {
			bounds,
			seed,
			path: Vec::new(),
			levels: Vec::new(),
			half_width: 0.0,
			thalweg_half: 0.0,
			skirt_half: 0.0,
			head_water: 0.0,
			toe_water: 0.0,
			modulations: Vec::new(),
			fills: Vec::new(),
		};

		let width_u = width_scale_u01(seed, min, params);
		let Some(budget) = StreamBandBudget::try_from_short_half(short_half, params, width_u) else {
			return empty;
		};

		let inset = budget.apron_half.max(budget.mu);
		let lo = min + Vec2::splat(inset);
		let hi = max - Vec2::splat(inset);
		if lo.x >= hi.x || lo.y >= hi.y {
			return empty;
		}

		let a0 = sample_endpoint(seed, ENDPOINT_A_SALT, lo, hi);
		let b0 = sample_endpoint(seed, ENDPOINT_B_SALT, lo, hi);
		if a0.distance(b0) < MIN_PATH_LEN * 0.5 {
			return empty;
		}

		let (head_xz, _, toe_xz, _) = DownhillPair::order(a0, b0, height_at);
		let path = params.spine.build(bounds, seed.wrapping_add(21), head_xz, toe_xz);
		if path.len() < 2 {
			return empty;
		}
		let path_len: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
		if path_len < MIN_PATH_LEN {
			return empty;
		}

		let levels = node_water_levels(&path, height_at, params.water_sink, params.min_drop);
		let head_water = levels.first().copied().unwrap_or(0.0);
		let toe_water = levels.last().copied().unwrap_or(head_water);

		let half_w = budget.half_width;
		let thalweg_w = budget.thalweg_half;
		let skirt_w = budget.skirt_half;
		let apron_w = budget.apron_half;
		let rim_lift = params.rim_lift.max(0.0);
		let bank = bank_levels(&levels, rim_lift);
		let step_guess = params
			.spine
			.walk_config(bounds)
			.step_len
			.max(half_w);
		let node_blend = if params.node_blend > 0.0 {
			params.node_blend
		} else {
			(step_guess * 0.45).max(half_w * 0.5)
		};

		let depth = params.depth * (0.7 + 0.6 * n01_at(seed, 0x57EA_DE07, min));
		let shore_amp = (half_w * params.shore_indent_frac.clamp(0.0, 0.4)).max(0.01);
		let shore_freq = params.shore_freq.max(1.5 / half_w.max(1.0)).clamp(1.0e-4, 0.14);
		let shore_noise = RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp);

		// Size-scaled apron / rim noise (authored freqs @ NOISE_FREQ_REF_RADIUS).
		// Channel half-width is the stream analog of lake short_water.
		let apron_band = (apron_w - skirt_w).max(0.5);
		let apron_frac_lo = params
			.apron_indent_frac_min
			.min(params.apron_indent_frac_max)
			.clamp(0.0, 0.5);
		let apron_frac_hi = params
			.apron_indent_frac_min
			.max(params.apron_indent_frac_max)
			.clamp(0.0, 0.5);
		let apron_freq_lo = params.apron_freq_min.min(params.apron_freq_max).max(0.0);
		let apron_freq_hi = params.apron_freq_min.max(params.apron_freq_max).max(0.0);
		let apron_indent_frac =
			apron_frac_lo + (apron_frac_hi - apron_frac_lo) * n01_at(seed, APRON_AMP_SALT, min);
		let apron_amp = (apron_band * apron_indent_frac).max(0.01);
		let apron_freq_authored =
			apron_freq_lo + (apron_freq_hi - apron_freq_lo) * n01_at(seed, APRON_FREQ_SALT, min);
		let apron_freq =
			scale_noise_freq(apron_freq_authored, half_w, params.noise_freq_power);
		let apron_noise = RegionNoise::from_seed(seed.wrapping_add(6), apron_freq, apron_amp);

		let rim_amp_lo = params.rim_height_amp_min.min(params.rim_height_amp_max).max(0.0);
		let rim_amp_hi = params.rim_height_amp_min.max(params.rim_height_amp_max).max(0.0);
		let rim_freq_lo = params.rim_height_freq_min.min(params.rim_height_freq_max).max(0.0);
		let rim_freq_hi = params.rim_height_freq_min.max(params.rim_height_freq_max).max(0.0);
		let rim_height_amp =
			rim_amp_lo + (rim_amp_hi - rim_amp_lo) * n01_at(seed, RIM_HEIGHT_AMP_SALT, min);
		let rim_freq_authored =
			rim_freq_lo + (rim_freq_hi - rim_freq_lo) * n01_at(seed, RIM_HEIGHT_FREQ_SALT, min);
		let rim_height_freq =
			scale_noise_freq(rim_freq_authored, half_w, params.noise_freq_power);
		let rim_height = RegionNoise::from_seed(
			seed.wrapping_add(7),
			rim_height_freq,
			rim_height_amp,
		);
		let depth_noise = RegionNoise::from_seed(
			seed.wrapping_add(9),
			(1.4 / half_w.max(1.0)).clamp(0.04, 0.2),
			params.depth_noise_amp.max(0.0),
		);

		let apron_region = Region2D::Polyline(PolylineRegion::new(path.clone(), apron_w));
		let channel_region = Region2D::Polyline(PolylineRegion::new(path.clone(), half_w));
		let thalweg_region = Region2D::Polyline(PolylineRegion::new(path.clone(), thalweg_w));

		let apron_fade = ((apron_w - skirt_w) * 0.85).max(1.0);
		let skirt = JerseyModulation::PolylineGrading(
			RegionPolylineGradingModulation::new(
				apron_region,
				path.clone(),
				bank,
				0.0,
				apron_fade,
			)
			.with_node_blend(node_blend)
			.with_noise(apron_noise)
			.with_height_noise_add_only(rim_height)
			.raise_only(),
		);

		// Floor grade sits a freeboard below W across the full channel so the
		// exact W−h fill slab has continuous thickness between path nodes.
		// No boundary / height noise on this pass — those pinched the corridor
		// and left mid-segment beds at or above W.
		let freeboard = params.channel_freeboard.max(0.25);
		let bed = bed_levels(&levels, freeboard);
		let channel_fade = (half_w * 0.15).max(0.35).min(half_w * 0.35);
		let channel = JerseyModulation::PolylineGrading(
			RegionPolylineGradingModulation::new(
				channel_region.clone(),
				path.clone(),
				bed,
				0.0,
				channel_fade,
			)
			.with_node_blend(node_blend)
			.depression_only(),
		);

		let thalweg_fade = (thalweg_w * 0.35).max(0.4);
		let thalweg = JerseyModulation::Affine(
			RegionAffineModulation::new(thalweg_region, 1.0, -depth, 0.0, thalweg_fade)
				.with_noise(shore_noise.clone())
				.with_height_noise(depth_noise),
		);

		// Extra relative cut on the wet channel (below the freeboard bed).
		let channel_cut = JerseyModulation::Affine(
			RegionAffineModulation::new(
				channel_region,
				1.0,
				-(freeboard * 0.25 + depth * 0.1),
				0.0,
				channel_fade,
			)
			.with_noise(shore_noise.clone()),
		);

		// Fill is a graded free-surface half-space on the shared terrain lattice.
		// Support is intentionally wider than the carve; undercut keeps bank columns wet.
		let fill_fade = params.shore_fade.max(0.25);
		let fill_half = (half_w * params.fill_half_width_scale.max(1.0)).max(half_w);
		let fill_region = Region2D::Polyline(PolylineRegion::new(path.clone(), fill_half));
		let fill = WaterFill {
			region: fill_region,
			inner_radius: 0.0,
			outer_radius: fill_fade,
			noise: None,
			surface: WaterSurface::Graded {
				path: path.clone(),
				levels: levels.clone(),
				node_blend,
			},
			terrain_undercut: params.fill_undercut.max(0.0),
		};

		Self {
			bounds,
			seed,
			path,
			levels,
			half_width: half_w,
			thalweg_half: thalweg_w,
			skirt_half: skirt_w,
			head_water,
			toe_water,
			modulations: vec![skirt, channel, channel_cut, thalweg],
			fills: vec![fill],
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, StreamParams::default(), None)
	}

	pub fn is_empty(&self) -> bool {
		self.modulations.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 20.0, 20.0);
		let stream = Stream::from_bounds_default(bounds, 11);
		assert!(stream.is_empty());
		Ok(())
	}

	#[test]
	fn graded_fill_decreases_along_path() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.05 * x - 0.01 * z;
		let stream = Stream::from_bounds(bounds, 42, StreamParams::default(), Some(&height));
		assert!(!stream.is_empty());
		assert!(stream.head_water > stream.toe_water);
		let fill = stream.fills.first().expect("fill");
		let head = *stream.path.first().expect("path head");
		let toe = *stream.path.last().expect("path toe");
		let w_head = fill.surface_level_at(head.x, head.y);
		let w_toe = fill.surface_level_at(toe.x, toe.y);
		assert!(
			w_head > w_toe + 0.2,
			"graded fill should drop along path: {w_head} → {w_toe}"
		);
		assert!(fill.softmask_at(head.x, head.y) < 0.5);
		let far = Vec2::new(bounds.min.x - 80.0, bounds.min.y - 80.0);
		assert!(fill.softmask_at(far.x, far.y) >= 1.0);
		Ok(())
	}

	#[test]
	fn fill_support_is_liberal_vs_channel() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.05 * x - 0.01 * z;
		let params = StreamParams::default();
		let stream = Stream::from_bounds(bounds, 42, params, Some(&height));
		assert!(!stream.is_empty());
		let fill = stream.fills.first().expect("fill");
		assert!(fill.terrain_undercut >= params.fill_undercut - 1e-3);
		assert!(fill.noise.is_none());
		match &fill.region {
			Region2D::Polyline(poly) => {
				let expected = stream.half_width * params.fill_half_width_scale.max(1.0);
				assert!(
					(poly.half_width - expected).abs() < 1e-3,
					"fill half {} vs expected liberal {}",
					poly.half_width,
					expected
				);
				assert!(poly.half_width > stream.half_width);
			}
			other => panic!("expected polyline fill region, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn channel_bed_below_w_along_segments() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.04 * x - 0.02 * z;
		let params = StreamParams::default();
		let freeboard = params.channel_freeboard;
		let stream = Stream::from_bounds(bounds, 19, params, Some(&height));
		assert!(!stream.is_empty());
		let fill = stream.fills.first().expect("fill");
		assert!(stream.path.len() >= 2);
		for window in stream.path.windows(2) {
			let mid = window[0].lerp(window[1], 0.5);
			let w = fill.surface_level_at(mid.x, mid.y);
			let mut h = height(mid.x, mid.y);
			for m in &stream.modulations {
				h = m.modify_elevation(h, mid.x, mid.y);
			}
			assert!(
				h < w - freeboard * 0.5,
				"mid-segment bed {h} should sit under W {w} (freeboard {freeboard})"
			);
			assert!(
				fill.wet_y_span_at(mid.x, mid.y, h).is_some(),
				"exact fill should have volume mid-segment"
			);
		}
		Ok(())
	}

	#[test]
	fn fill_tracks_piecewise_node_samples() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		// Steep then flat — a global head→toe grade would mis-place the mid node.
		let height = |x: f32, _z: f32| {
			if x < 200.0 {
				120.0 - 0.2 * x
			} else {
				80.0
			}
		};
		let mut params = StreamParams::default();
		params.min_drop = 0.0;
		params.water_sink = 0.0;
		let stream = Stream::from_bounds(bounds, 7, params, Some(&height));
		assert!(!stream.is_empty());
		assert_eq!(stream.path.len(), stream.levels.len());
		let fill = stream.fills.first().expect("fill");
		for (p, &w) in stream.path.iter().zip(stream.levels.iter()) {
			let got = fill.surface_level_at(p.x, p.y);
			assert!(
				(got - w).abs() < 0.15,
				"node ({}, {}) W={got} expected {w}",
				p.x,
				p.y
			);
		}
		// Interior segment sample should sit between its endpoints.
		if stream.path.len() >= 2 {
			let a = stream.path[0];
			let b = stream.path[1];
			let mid = a.lerp(b, 0.5);
			let w_mid = fill.surface_level_at(mid.x, mid.y);
			let lo = stream.levels[0].min(stream.levels[1]);
			let hi = stream.levels[0].max(stream.levels[1]);
			assert!(w_mid >= lo - 0.2 && w_mid <= hi + 0.2);
		}
		Ok(())
	}

	#[test]
	fn skirt_uses_add_only_rim_with_lake_scale_defaults() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let height = |x: f32, z: f32| 100.0 - 0.05 * x - 0.01 * z;
		let params = StreamParams::default();
		assert!(params.rim_height_amp_max >= 15.0);
		assert!(params.apron_indent_frac_max > params.apron_indent_frac_min);
		let stream = Stream::from_bounds(bounds, 42, params, Some(&height));
		assert!(!stream.is_empty());
		assert_eq!(stream.modulations.len(), 4);
		// Skirt is first; raise-only bank grade should not lower far-field terrain.
		let far = Vec2::new(bounds.min.x - 120.0, bounds.min.y - 120.0);
		let h0 = height(far.x, far.y);
		let h1 = stream.modulations[0].modify_elevation(h0, far.x, far.y);
		assert!((h1 - h0).abs() < 1e-3);
		Ok(())
	}
}
