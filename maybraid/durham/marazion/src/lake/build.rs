//! Assemble lake bowl depression + shared plateau apron from a laid-out footprint.

use crate::apron::{jittered_depth, ApronNoiseSalts};
use crate::complex::{WatershedApronShelf, WatershedDepressionComplex};
use crate::depression::{WatershedDepression, WatershedDepressionKind};
use crate::fill::{WaterFill, WaterSurface};
use crate::lake::budget::LakeBandBudget;
use crate::lake::shelf::ShelfLevels;
use crate::lake::LakeParams;
use crate::noise::scale_noise_freq;
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	EllipseRegion, JerseyModulation, Region2D, RegionBowlModulation, RegionNoise,
};
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

/// Lake-specific stamp: one bowl depression + its shared apron.
///
/// Convert with [`Self::into_complex`] → [`WatershedDepressionComplex`].
#[derive(Debug, Clone)]
pub(crate) struct LakeBowl {
	pub depression: WatershedDepression,
	pub apron: WatershedApronShelf,
	pub fill_radius: f32,
}

impl LakeBowl {
	/// `LakeBowl` → sole-node [`WatershedDepressionComplex`].
	pub fn into_complex(self, bounds: Bounds2, seed: u32) -> WatershedDepressionComplex {
		self.depression.into_complex(bounds, seed, self.apron)
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
	let rim_w = budget.rim_width;
	let apron_w = budget.apron_width.max(1.0);
	let rotation = budget.rotation;
	let short_water = budget.water_radius();
	let water_level = layout.levels.water_level;
	let rim_level = layout.levels.rim_level;

	let depth = jittered_depth(seed, DEPTH_SALT, anchor, params.depth, 0.65, 0.7);
	let bowl_fade = (rim_w * 0.25).max(0.5).min(short_water * 0.2);

	let rim_bleed = rim_w * params.rim_bleed_frac.max(0.0);
	let fill_r = water_r + Vec2::splat(rim_bleed);
	let max_fill = plateau_r + Vec2::splat(apron_w - 0.5);
	let fill_r = Vec2::new(fill_r.x.min(max_fill.x), fill_r.y.min(max_fill.y)).max(water_r);

	let plateau_region = ellipse_region(center, plateau_r, rotation);
	let water_region = ellipse_region(center, water_r, rotation);
	let fill_region = ellipse_region(center, fill_r, rotation);

	let shore_amp = (short_water * params.shore_indent_frac.clamp(0.0, 0.45))
		.min(rim_w * 0.85)
		.max(0.01);
	let shore_freq =
		scale_noise_freq(params.shore_freq, short_water, params.apron.noise_freq_power);
	let shore_noise = RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp);

	let apron_noise = params.apron.sample_noise(
		seed,
		anchor,
		apron_w,
		short_water,
		ApronNoiseSalts::LAKE,
	);
	let apron_outer = apron_w + apron_noise.apron_amp;

	let depth_noise_freq =
		scale_noise_freq(params.depth_noise_freq, short_water, params.apron.noise_freq_power);
	let depth_noise = RegionNoise::from_seed(
		seed.wrapping_add(9),
		depth_noise_freq,
		params.depth_noise_amp.max(0.0),
	);

	let undercut = params.terrain_undercut.max(0.0);
	let bed_ceiling = (rim_level + params.island_lift.max(0.0))
		.max(water_level + undercut + params.depth_noise_amp.max(0.0) * 0.85);
	let shore_frac = params.depth_shore_frac.clamp(0.0, 1.0);
	let center_bed = water_level - depth;
	let shore_bed = water_level - depth * shore_frac;
	let bowl = JerseyModulation::Bowl(
		RegionBowlModulation::new(
			water_region.clone(),
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
		outer_radius: params.shore_fade.max(1.0),
		noise: Some(shore_noise),
		surface: WaterSurface::Flat { level: water_level },
		terrain_undercut: undercut,
	};

	let depression = WatershedDepression::new(
		WatershedDepressionKind::LakeBowl,
		water_region,
		vec![bowl],
		Some(fill),
	);
	let apron = WatershedApronShelf::LakeFlatten {
		region: plateau_region,
		rim_level,
		outer_radius: apron_outer,
		apron_noise: apron_noise.apron,
		rim_height: apron_noise.rim_height,
	};

	LakeBowl {
		depression,
		apron,
		fill_radius: fill_r.min_element(),
	}
}
