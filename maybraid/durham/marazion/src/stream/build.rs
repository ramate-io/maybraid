//! Assemble stream corridor depression + shared apron from a laid-out path.

use crate::apron::{jittered_depth, ApronNoiseSalts};
use crate::complex::WatershedApronShelf;
use crate::depression::{WatershedDepression, WatershedDepressionKind};
use crate::fill::{WaterFill, WaterSurface};
use crate::stream::path::{bank_levels, bed_levels};
use crate::stream::{StreamBandBudget, StreamParams};
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	JerseyModulation, PolylineRegion, RegionAffineModulation, Region2D, RegionNoise,
	RegionPolylineGradingModulation,
};
use procedural_common::Bounds2;

const DEPTH_SALT: u32 = 0x57EA_DE07;

/// Laid-out stream geometry ready for depression / apron / fill construction.
pub(crate) struct StreamLayout {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub budget: StreamBandBudget,
	pub node_blend: f32,
}

/// Wet-core corridor + shared raise-only apron shelf.
pub(crate) struct StreamStampParts {
	pub depression: WatershedDepression,
	pub apron: WatershedApronShelf,
}

pub(crate) fn resolve_node_blend(
	params: StreamParams,
	bounds: Bounds2,
	half_w: f32,
) -> f32 {
	if params.node_blend > 0.0 {
		params.node_blend
	} else {
		let step_guess = params.spine.walk_config(bounds).step_len.max(half_w);
		(step_guess * 0.45).max(half_w * 0.5)
	}
}

pub(crate) fn build_parts(
	seed: u32,
	anchor: Vec2,
	params: StreamParams,
	layout: &StreamLayout,
) -> StreamStampParts {
	let path = &layout.path;
	let levels = &layout.levels;
	let half_w = layout.budget.half_width;
	let thalweg_w = layout.budget.thalweg_half;
	let skirt_w = layout.budget.skirt_half;
	let apron_w = layout.budget.apron_half;
	let node_blend = layout.node_blend;

	let depth = jittered_depth(seed, DEPTH_SALT, anchor, params.depth, 0.7, 0.6);
	let shore_amp = (half_w * params.shore_indent_frac.clamp(0.0, 0.4)).max(0.01);
	let shore_freq = params.shore_freq.max(1.5 / half_w.max(1.0)).clamp(1.0e-4, 0.14);
	let shore_noise = RegionNoise::from_seed(seed.wrapping_add(5), shore_freq, shore_amp);

	let apron_band = (apron_w - skirt_w).max(0.5);
	let apron_noise = params.apron.sample_noise(
		seed,
		anchor,
		apron_band,
		half_w,
		ApronNoiseSalts::STREAM,
	);
	let depth_noise = RegionNoise::from_seed(
		seed.wrapping_add(9),
		(1.4 / half_w.max(1.0)).clamp(0.04, 0.2),
		params.depth_noise_amp.max(0.0),
	);

	let channel_region = Region2D::Polyline(PolylineRegion::new(path.clone(), half_w));
	let thalweg_region = Region2D::Polyline(PolylineRegion::new(path.clone(), thalweg_w));
	let apron_region = Region2D::Polyline(PolylineRegion::new(path.clone(), apron_w));

	let freeboard = params.channel_freeboard.max(0.25);
	let bed = bed_levels(levels, freeboard);
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

	let channel_cut = JerseyModulation::Affine(
		RegionAffineModulation::new(
			channel_region.clone(),
			1.0,
			-(freeboard * 0.25 + depth * 0.1),
			0.0,
			channel_fade,
		)
		.with_noise(shore_noise),
	);

	let fill_half = (half_w * params.fill_half_width_scale.max(1.0)).max(half_w);
	let fill = WaterFill {
		region: Region2D::Polyline(PolylineRegion::new(path.clone(), fill_half)),
		inner_radius: 0.0,
		outer_radius: params.shore_fade.max(0.25),
		noise: None,
		surface: WaterSurface::Graded {
			path: path.clone(),
			levels: levels.clone(),
			node_blend,
		},
		terrain_undercut: params.fill_undercut.max(0.0),
	};

	let depression = WatershedDepression::new(
		WatershedDepressionKind::StreamCorridor,
		channel_region,
		vec![channel, channel_cut, thalweg],
		Some(fill),
	);

	let apron = WatershedApronShelf::StreamRaiseOnly {
		region: apron_region,
		path: path.clone(),
		bank_levels: bank_levels(levels, params.rim_lift),
		node_blend,
		fade: ((apron_w - skirt_w) * 0.85).max(1.0),
		apron_noise: apron_noise.apron,
		rim_height: apron_noise.rim_height,
	};

	StreamStampParts { depression, apron }
}
