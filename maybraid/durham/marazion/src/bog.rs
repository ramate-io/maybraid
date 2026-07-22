//! Bog pocket water — lake bowl with post-carve basin backfill.
//!
//! Not the RFC-127 §3.1.3.3 micro-lake lattice. Instead: stamp a normal lake
//! (carve + fill + apron), then add [`crate::backfill::WatershedBackfill`]
//! elevation noise inside the wet core so the bed reads hummocky / islanded
//! after water is placed.

use crate::backfill::BasinBackfillParams;
use crate::fill::WaterFill;
use crate::lake::build::{build_bowl, LakeLayout};
use crate::lake::shelf::{
	aspect_u01, planned_center as planned_center_impl, rim_width_u01, rotation_u11, shelf_levels,
	water_scale_u01,
};
use crate::lake::{LakeBandBudget, LakeParams};
use crate::noise::scale_noise_freq;
use bevy_math::Vec2;
use jersey_terrain_stamps::JerseyModulation;
use procedural_common::Bounds2;

/// Salt offset for basin backfill noise draws.
const BASIN_BACKFILL_SALT: u32 = 0xB06_BACF1;

/// Authoring knobs: lake footprint/bowl + basin backfill height noise.
#[derive(Debug, Clone, Copy)]
pub struct BogParams {
	/// Lake bowl / apron / fill knobs (bog-tuned defaults are shallower).
	pub lake: LakeParams,
	/// Post-carve basin height noise over the wet core.
	pub basin: BasinBackfillParams,
}

impl Default for BogParams {
	fn default() -> Self {
		let mut lake = LakeParams::default();
		// Shallower, near-flat floor so basin backfill crests across the wet core.
		lake.depth = 7.0;
		lake.depth_noise_amp = 2.0;
		lake.depth_shore_frac = 0.9;
		Self {
			lake,
			basin: BasinBackfillParams {
				// Tall enough to crest above W as islands / hummocks.
				amp: 12.0,
				// Higher base frequency → denser mound field across the wet core.
				freq: 0.06,
				fade: 2.5,
				octaves: 3,
				add_only: true,
			},
		}
	}
}

/// Bog stamp products for one pocket-water leaf (lake-shaped public surface).
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
	pub modulations: Vec<JerseyModulation>,
	pub fills: Vec<WaterFill>,
}

impl Bog {
	pub fn planned_center(bounds: Bounds2, seed: u32, params: BogParams) -> Vec2 {
		planned_center_impl(bounds, seed, params.lake)
	}

	fn empty(bounds: Bounds2, seed: u32, center: Vec2) -> Self {
		Self {
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
		}
	}

	/// Build a lake bowl + basin backfill, or an empty stamp when the leaf is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: BogParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let min = bounds.min;
		let center = Self::planned_center(bounds, seed, params);
		let lake_p = params.lake;
		let u = water_scale_u01(seed, min, lake_p);
		let rim_u = rim_width_u01(seed, min, lake_p);
		let asp = aspect_u01(seed, min);
		let rot = rotation_u11(seed, min);
		let Some(budget) =
			LakeBandBudget::try_inscribed(bounds, center, lake_p, u, rim_u, asp, rot)
		else {
			return Self::empty(
				bounds,
				seed,
				Vec2::new(
					(bounds.min.x + bounds.max.x) * 0.5,
					(bounds.min.y + bounds.max.y) * 0.5,
				),
			);
		};

		let levels = shelf_levels(seed, min, center, &budget, lake_p, height_at);
		let layout = LakeLayout {
			center,
			budget,
			levels,
		};
		let bowl = build_bowl(seed, min, lake_p, &layout);
		let fill_radius = bowl.fill_radius;
		let wet_core = bowl.depression.wet_core.clone();

		let short_water = layout.budget.water_radius();
		let basin_freq = scale_noise_freq(
			params.basin.freq,
			short_water,
			lake_p.apron.noise_freq_power,
		);
		let basin = BasinBackfillParams {
			freq: basin_freq,
			..params.basin
		}
		.sample(seed, BASIN_BACKFILL_SALT, wet_core);

		let compiled = bowl
			.into_complex(bounds, seed)
			.with_backfill(basin)
			.compile();

		Self {
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
			modulations: compiled.modulations,
			fills: compiled.fills,
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, BogParams::default(), None)
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
		let bog = Bog::from_bounds_default(bounds, 11);
		assert!(bog.is_empty());
		Ok(())
	}

	#[test]
	fn from_bounds_non_empty_ends_with_backfill() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let bog = Bog::from_bounds(bounds, 11, BogParams::default(), Some(&|_, _| 40.0));
		assert!(!bog.is_empty());
		assert!(!bog.fills.is_empty());
		// Apron + bowl + basin backfill.
		assert!(
			bog.modulations.len() >= 3,
			"expected apron+bowl+backfill, got {}",
			bog.modulations.len()
		);

		let base = 40.0;
		let last = bog.modulations.last().expect("backfill");
		// Identity outside the basin: last op alone should not move far-field.
		let outside = bog.center
			+ Vec2::new(bog.plateau_radius + bog.apron_width + 40.0, 0.0);
		let h_out = last.modify_elevation(base, outside.x, outside.y);
		assert!(
			(h_out - base).abs() < 1e-3,
			"final backfill should be near-identity outside basin: {h_out}"
		);

		// Inside wet core, raise-only backfill should lift height.
		let h_in = last.modify_elevation(base, bog.center.x, bog.center.y);
		assert!(
			h_in > base + 0.05,
			"final backfill should raise basin (mounds): {h_in} vs {base}"
		);
		Ok(())
	}

	#[test]
	fn bog_keeps_water_fill() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 320.0, 320.0);
		let bog = Bog::from_bounds_default(bounds, 19);
		assert!(!bog.is_empty());
		assert_eq!(bog.fills.len(), 1);
		Ok(())
	}
}
