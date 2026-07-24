//! Assemble lake bowl as a hydrology node (ellipse + radial bowl).

use crate::authored::apron::{jittered_depth, sample_apron_rim_noise, ApronNoiseSalts};
use crate::authored::lake::budget::LakeBandBudget;
use crate::authored::lake::shelf::ShelfLevels;
use crate::authored::lake::LakeParams;
use crate::authored::noise::{scale_noise_freq, NOISE_FREQ_REF_RADIUS};
use crate::primitive::complex::HydroComplex;
use crate::primitive::hydro::{
	Ellipse, HydroElevation, HydroFootprint, HydroPrimitive, RadialBowl,
};
use crate::primitive::node::HydroNode;
use crate::primitive::parameters::{
	HydroParams, DISABLE_RIM_LIFT, DISABLE_SHORE_BOUNDARY_NOISE, TARGET_RIM_WIDTH,
};
use bevy_math::Vec2;
use jersey_terrain_stamps::{EllipseRegion, Region2D, RegionNoise};
use procedural_common::Bounds2;

const DEPTH_SALT: u32 = 0x1A7E_DE07;

fn ellipse_region(center: Vec2, radii: Vec2, rotation: f32) -> Region2D {
	Region2D::Ellipse(EllipseRegion {
		center,
		radii: radii.max(Vec2::splat(1e-3)),
		rotation,
	})
}

/// Laid-out lake footprint + vertical levels.
pub(crate) struct LakeLayout {
	pub center: Vec2,
	pub budget: LakeBandBudget,
	pub levels: ShelfLevels,
}

/// Lake-specific stamp: one radial-bowl hydrology node.
///
/// Convert with [`Self::into_complex`] → [`HydroComplex`].
#[derive(Debug, Clone)]
pub(crate) struct LakeBowl {
	/// Wet footprint (basin backfill / overlays).
	pub wet_core: Region2D,
	pub node: HydroNode,
	/// Authoring metadata: water radius + rim bleed (water SDF follows carve \(\phi\)).
	pub fill_radius: f32,
}

impl LakeBowl {
	/// `LakeBowl` → sole-node [`HydroComplex`] with hydrology emit.
	pub fn into_complex(self, bounds: Bounds2, seed: u32) -> HydroComplex {
		HydroComplex::new(bounds, seed).with_hydro(vec![self.node])
	}
}

pub(crate) fn build_bowl(
	seed: u32,
	anchor: Vec2,
	params: LakeParams,
	layout: &LakeLayout,
) -> LakeBowl {
	let center = layout.center;
	let budget = &layout.budget;
	let water_r = budget.water_radii;
	let plateau_r = budget.plateau_radii;
	let rim_w = TARGET_RIM_WIDTH;
	let apron_w = budget.apron_width.max(1.0);
	let rotation = budget.rotation;
	let short_water = budget.water_radius();
	let water_level = layout.levels.water_level;

	// Deeper bowls for larger lakes: `params.depth` is the centroid depth at
	// [`NOISE_FREQ_REF_RADIUS`].
	let depth_scaled = params.depth.max(0.25)
		* (short_water / NOISE_FREQ_REF_RADIUS).clamp(0.35, 4.0);
	let depth = jittered_depth(seed, DEPTH_SALT, anchor, depth_scaled, 0.65, 0.7);

	let rim_bleed = rim_w * params.rim_bleed_frac.max(0.0);
	let fill_r = water_r + Vec2::splat(rim_bleed);
	let max_fill = plateau_r + Vec2::splat(apron_w - 0.5);
	let fill_r = Vec2::new(fill_r.x.min(max_fill.x), fill_r.y.min(max_fill.y)).max(water_r);

	let water_region = ellipse_region(center, water_r, rotation);

	let apron_noise = sample_apron_rim_noise(
		&params.apron,
		&params.rim,
		seed,
		anchor,
		apron_w,
		short_water,
		ApronNoiseSalts::LAKE,
	);
	let apron_outer = (apron_w + apron_noise.apron_amp).max(apron_w);
	let shore_amp = if DISABLE_SHORE_BOUNDARY_NOISE {
		0.0
	} else {
		(short_water.max(1.0) * params.shore_indent_frac.clamp(0.0, 0.45)).max(0.01)
	};
	let boundary_noise = if DISABLE_SHORE_BOUNDARY_NOISE {
		None
	} else {
		let shore_freq = scale_noise_freq(
			params.shore_freq.max(0.0),
			short_water,
			params.apron.noise_freq_power,
		);
		Some(RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp))
	};

	let max_correction_extent = (rim_w + apron_outer + shore_amp).max(0.0);
	let mut rim = params.rim;
	rim.width = rim_w;
	rim.lift = if DISABLE_RIM_LIFT {
		0.0
	} else {
		params.rim.lift.max(0.0)
	};
	rim.shelf_anchor = Some(layout.levels.shelf_anchor);
	rim.uplift_cap = params.rim.recipe_uplift_cap();
	let mut apron = params.apron;
	apron.width = apron_outer;

	let hydro_params = HydroParams {
		rim,
		apron,
		rim_height: apron_noise.rim_height,
		boundary_noise,
		shore_blend: HydroParams::recommend_shore_blend(rim_w, shore_amp),
		rim_apron_blend: HydroParams::recommend_shore_blend(rim_w, shore_amp),
	};
	let node = HydroNode::new(
		HydroPrimitive {
			footprint: HydroFootprint::Ellipse(Ellipse {
				center,
				radii: water_r.max(Vec2::splat(1e-3)),
				rotation,
			}),
			elevation: HydroElevation::Radial(RadialBowl {
				surface: water_level,
				center_depth: depth.max(0.25),
			}),
			influence_pad: max_correction_extent,
		},
		hydro_params,
		max_correction_extent,
	);

	LakeBowl {
		wet_core: water_region,
		node,
		fill_radius: fill_r.min_element(),
	}
}
