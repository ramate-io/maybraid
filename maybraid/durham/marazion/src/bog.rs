//! Bog pocket water — lake bowl with post-carve basin backfill.
//!
//! Not the RFC-127 §3.1.3.3 micro-lake lattice. Instead: stamp a normal lake
//! (hydro radial bowl + fill + shared apron), then add
//! [`crate::backfill::WatershedBackfill`] elevation noise inside the wet core so
//! the bed reads hummocky / islanded after water is placed.
//!
//! Basin noise itself is depth-incentive ([`BasinBackfillParams::depth_frac`]);
//! [`BogBasinFill`] chooses how aggressively that freeboard is filled / crested.

use crate::backfill::{BasinBackfillParams, WatershedBackfill};
use crate::complex::WatershedDepressionComplex;
use crate::lake::build::{build_bowl, LakeBowl, LakeLayout};
use crate::lake::shelf::{
	aspect_u01, planned_center as planned_center_impl, rim_width_u01, rotation_u11, shelf_levels,
	water_scale_u01,
};
use crate::lake::{LakeBandBudget, LakeParams};
use crate::noise::scale_noise_freq;
use bevy_math::Vec2;
use procedural_common::Bounds2;

/// Salt offset for basin backfill noise draws.
const BASIN_BACKFILL_SALT: u32 = 0xB06_BACF1;

/// How a bog fills its basin relative to bowl freeboard / \(W\).
///
/// Converts to a [`BasinBackfillParams::depth_frac`] so backfill stays
/// depth-incentive while bog owns peaking taste.
#[derive(Debug, Clone, Copy)]
pub struct BogBasinFill {
	/// Target crest height above \(W\) at full-scale raise-only noise.
	pub peak_above_w: f32,
	/// Unit-noise fraction (`|sample| / amp`) at which the bed reaches \(W\).
	/// Higher → gentler mounds / fewer hard peaks. Clamped to `(0.05, 1]`.
	pub crest_unit: f32,
}

impl Default for BogBasinFill {
	fn default() -> Self {
		Self {
			// Short islands — frequent but not tall (~15% shorter than prior).
			peak_above_w: 0.95,
			crest_unit: 0.4,
		}
	}
}

impl BogBasinFill {
	/// Depth-incentive fraction: `amp / freeboard`.
	pub fn depth_frac(&self, freeboard: f32) -> f32 {
		let fb = freeboard.max(1e-3);
		let u = self.crest_unit.clamp(0.05, 1.0);
		let peak = self.peak_above_w.max(0.0);
		let amp = (fb / u).max(fb + peak);
		amp / fb
	}
}

/// Authoring knobs: lake footprint/bowl + basin noise + bog fill policy.
#[derive(Debug, Clone, Copy)]
pub struct BogParams {
	/// Lake bowl / apron / fill knobs (bog-tuned defaults are shallow + flat).
	pub lake: LakeParams,
	/// Spatial basin backfill knobs (`depth_frac` is set from [`Self::fill`]).
	pub basin: BasinBackfillParams,
	/// Peaking / fill policy mapped onto basin freeboard.
	pub fill: BogBasinFill,
}

impl Default for BogParams {
	fn default() -> Self {
		let mut lake = LakeParams::default();
		// Short freeboard so depth-incentive fill crests often and gently.
		lake.depth = 2.2;
		lake.depth_noise_amp = 0.65;
		lake.depth_shore_frac = 0.95;
		Self {
			lake,
			basin: BasinBackfillParams {
				// Overwritten from `fill` at stamp time.
				depth_frac: 1.0,
				freq: 0.035,
				fade: 2.5,
				octaves: 2,
				add_only: true,
			},
			fill: BogBasinFill::default(),
		}
	}
}

/// Authored bog **plan**: lake-shaped layout + basin backfill recipe.
///
/// Realize with [`Self::into_complex`] into a [`WatershedDepressionComplex`]
/// (bowl + basin backfill). `None` from [`Self::from_bounds`] means the leaf
/// is too small.
#[derive(Debug, Clone)]
pub struct Bog {
	pub bounds: Bounds2,
	pub seed: u32,
	pub center: Vec2,
	pub water_radii: Vec2,
	pub rotation: f32,
	pub water_radius: f32,
	pub plateau_radius: f32,
	pub rim_width: f32,
	pub apron_width: f32,
	pub fill_radius: f32,
	pub water_level: f32,
	bowl: LakeBowl,
	basin: WatershedBackfill,
}

impl Bog {
	pub fn planned_center(bounds: Bounds2, seed: u32, params: BogParams) -> Vec2 {
		planned_center_impl(bounds, seed, params.lake)
	}

	/// Build a lake bowl + basin backfill plan, or `None` when the leaf is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: BogParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Option<Self> {
		let min = bounds.min;
		let center = Self::planned_center(bounds, seed, params);
		let lake_p = params.lake;
		let u = water_scale_u01(seed, min, lake_p);
		let rim_u = rim_width_u01(seed, min, lake_p);
		let asp = aspect_u01(seed, min);
		let rot = rotation_u11(seed, min);
		let budget =
			LakeBandBudget::try_inscribed(bounds, center, lake_p, u, rim_u, asp, rot)?;

		let levels = shelf_levels(seed, min, center, &budget, lake_p, height_at);
		let layout = LakeLayout { center, budget, levels };
		let bowl = build_bowl(seed, min, lake_p, &layout);
		let fill_radius = bowl.fill_radius;
		let wet_core = bowl.wet_core.clone();

		let short_water = layout.budget.water_radius();
		let basin_freq =
			scale_noise_freq(params.basin.freq, short_water, lake_p.apron.noise_freq_power);
		// Cover jittered_depth high end (~1.35× authored).
		let freeboard = lake_p.depth.max(0.0) * 1.35;
		let basin = BasinBackfillParams {
			depth_frac: params.fill.depth_frac(freeboard),
			freq: basin_freq,
			..params.basin
		}
		.sample_over_freeboard(freeboard, seed, BASIN_BACKFILL_SALT, wet_core);

		Some(Self {
			bounds,
			seed,
			center: layout.center,
			water_radii: layout.budget.water_radii,
			rotation: layout.budget.rotation,
			water_radius: layout.budget.water_radius(),
			plateau_radius: layout.budget.plateau_radius(),
			rim_width: layout.budget.rim_width,
			apron_width: layout.budget.apron_width,
			fill_radius,
			water_level: layout.levels.water_level,
			bowl,
			basin,
		})
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Option<Self> {
		Self::from_bounds(bounds, seed, BogParams::default(), None)
	}

	/// Hydrology nodes authored by this bog (lake-shaped bowl; backfill is separate).
	pub fn hydrology_nodes(&self) -> Vec<crate::node::HydrologyNode> {
		vec![self.bowl.node.clone()]
	}

	/// Realize this plan as a sole-node complex with basin backfill.
	pub fn into_complex(self) -> WatershedDepressionComplex {
		self.bowl
			.into_complex(self.bounds, self.seed)
			.with_backfill(self.basin)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fill::WaterSurface;

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 20.0, 20.0);
		assert!(Bog::from_bounds_default(bounds, 11).is_none());
		Ok(())
	}

	#[test]
	fn from_bounds_hydro_plus_backfill() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let bog = Bog::from_bounds(bounds, 11, BogParams::default(), Some(&|_, _| 40.0)).expect("bog");
		let compiled = bog.clone().into_complex().compile();
		assert!(compiled.hydro.is_some());
		assert!(!compiled.fills.is_empty());
		assert!(matches!(
			compiled.fills[0].surface,
			WaterSurface::Hydro { .. }
		));
		assert_eq!(
			compiled.modulations.len(),
			1,
			"expected only post-hydro backfill jersey, got {}",
			compiled.modulations.len()
		);

		let base = 40.0;
		let last = compiled.modulations.last().expect("backfill");
		let outside = bog.center + Vec2::new(bog.plateau_radius + bog.apron_width + 40.0, 0.0);
		let h_out = last.modify_elevation(base, outside.x, outside.y);
		assert!(
			(h_out - base).abs() < 1e-3,
			"final backfill should be near-identity outside basin: {h_out}"
		);

		let h_in = last.modify_elevation(base, bog.center.x, bog.center.y);
		assert!(h_in > base + 0.05, "final backfill should raise basin (mounds): {h_in} vs {base}");
		Ok(())
	}

	#[test]
	fn bog_keeps_water_fill() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let bog = Bog::from_bounds_default(bounds, 19).expect("bog");
		assert_eq!(bog.clone().into_complex().compile().fills.len(), 1);
		Ok(())
	}

	#[test]
	fn fill_policy_sets_depth_frac_from_freeboard() -> anyhow::Result<()> {
		let params = BogParams::default();
		let freeboard = params.lake.depth * 1.35;
		let frac = params.fill.depth_frac(freeboard);
		let amp =
			BasinBackfillParams { depth_frac: frac, ..params.basin }.amp_for_freeboard(freeboard);
		let u = params.fill.crest_unit;
		assert!(
			(amp - (freeboard / u).max(freeboard + params.fill.peak_above_w)).abs() < 1e-3,
			"amp {amp} should match fill policy for freeboard {freeboard}"
		);
		// Milder than a tall-spike recipe; still depth-incentive.
		assert!(frac <= 2.5, "expected moderate depth_frac, got {frac}");
		assert!(frac > 1.5, "expected enough fill to crest often, got {frac}");
		Ok(())
	}

	#[test]
	fn some_interior_samples_crest_above_water() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let base = 40.0;
		let mut params = BogParams::default();
		params.lake.rotation_amp = 0.0;
		params.lake.aspect_strength = 0.0;
		params.lake.aspect_floor = 0.0;
		params.lake.depth_noise_amp = 0.0;
		let bog = Bog::from_bounds(bounds, 11, params, Some(&|_, _| base)).expect("bog");
		let compiled = bog.clone().into_complex().compile();

		let mut crested = 0usize;
		let mut samples = 0usize;
		let mut max_above = 0.0_f32;
		for &frac in &[0.15_f32, 0.35, 0.55, 0.7] {
			for i in 0..24 {
				let ang = i as f32 * std::f32::consts::TAU / 24.0;
				let p = bog.center + Vec2::new(ang.cos(), ang.sin()) * (bog.water_radius * frac);
				let h = compiled.modify_elevation(base, p.x, p.y);
				samples += 1;
				if h > bog.water_level + 0.15 {
					crested += 1;
					max_above = max_above.max(h - bog.water_level);
				}
			}
		}
		assert!(crested >= 3, "expected some island crests above W (crested={crested}/{samples})");
		// Mild fill: peaks should not tower far above W.
		assert!(max_above < 4.0, "crests should stay modest above W, got +{max_above}");
		Ok(())
	}
}
