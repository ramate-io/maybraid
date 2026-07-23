//! Assemble stream corridor as hydrology reach-segment nodes.

use crate::apron::{jittered_depth, shore_boundary_noise, ApronNoiseSalts, TARGET_RIM_WIDTH};
use crate::complex::HydrologyComplex;
use crate::depression::{WatershedDepression, WatershedDepressionKind};
use crate::node::{nodes_from_polyline, HydroParameters};
use crate::stream::{StreamBandBudget, StreamParams};
use bevy_math::Vec2;
use jersey_terrain_stamps::{PolylineRegion, Region2D};
use procedural_common::Bounds2;

const DEPTH_SALT: u32 = 0x57EA_DE07;

/// Laid-out stream geometry ready for depression / apron / fill construction.
pub(crate) struct StreamLayout {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub budget: StreamBandBudget,
}

/// Stream-specific stamp: one corridor as hydrology nodes.
///
/// Convert with [`Self::into_complex`] → [`HydrologyComplex`].
#[derive(Debug, Clone)]
pub(crate) struct StreamCorridor {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub half_width: f32,
	pub wet_core: Region2D,
	pub center_depth: f32,
	pub parameters: HydroParameters,
	pub max_correction_extent: f32,
}

impl StreamCorridor {
	/// `StreamCorridor` → sole-edge [`HydrologyComplex`].
	pub fn into_complex(self, bounds: Bounds2, seed: u32) -> HydrologyComplex {
		let nodes = nodes_from_polyline(
			&self.path,
			&self.levels,
			self.half_width,
			self.center_depth,
			&self.parameters,
			self.max_correction_extent,
		);
		let mut complex = HydrologyComplex::new(bounds, seed);
		let from = complex.push_node(crate::complex::WatershedNode::empty());
		let to = complex.push_node(crate::complex::WatershedNode::empty());
		complex.push_edge(crate::complex::WatershedEdge {
			from,
			to,
			depression: WatershedDepression::new(
				WatershedDepressionKind::StreamCorridor,
				self.wet_core,
			),
		});
		complex.with_hydrology(nodes)
	}
}

pub(crate) fn build_corridor(
	seed: u32,
	anchor: Vec2,
	params: StreamParams,
	layout: &StreamLayout,
) -> StreamCorridor {
	let path = layout.path.clone();
	let levels = layout.levels.clone();
	let half_w = layout.budget.half_width;
	let skirt_w = layout.budget.skirt_half;
	let apron_w = layout.budget.apron_half;

	let depth = jittered_depth(seed, DEPTH_SALT, anchor, params.depth, 0.7, 0.6);
	let freeboard = params.channel_freeboard.max(0.25);
	let center_depth = freeboard + depth;

	let apron_band = (apron_w - skirt_w).max(0.5);
	let apron_noise = params.apron.sample_noise(
		seed,
		anchor,
		apron_band,
		half_w,
		ApronNoiseSalts::STREAM,
	);

	let channel_region = Region2D::Polyline(PolylineRegion::new(path.clone(), half_w));
	let rim_w = TARGET_RIM_WIDTH;
	let apron_width = (apron_w - skirt_w).max(apron_band);
	let boundary_noise = shore_boundary_noise(
		seed,
		half_w,
		params.shore_indent_frac,
		params.shore_freq,
		params.apron.noise_freq_power,
	);
	let shore_amp = boundary_noise.noise.params().amplitude.abs();
	let max_correction_extent = (rim_w + apron_width + shore_amp).max(0.0);
	let rim_uplift_cap = params
		.apron
		.rim_height_amp_max
		.max(params.apron.rim_height_amp_min)
		.max(0.0);
	let parameters = HydroParameters {
		shelf_anchor: None,
		rim_lift: params.rim_lift.max(0.0),
		rim_width: rim_w,
		apron_width,
		rim_height: apron_noise.rim_height,
		rim_uplift_cap,
		boundary_noise: Some(boundary_noise),
		shore_fade: params.shore_fade.max(0.25),
		fill_undercut: params.fill_undercut.max(0.0),
	};

	StreamCorridor {
		path,
		levels,
		half_width: half_w,
		wet_core: channel_region,
		center_depth,
		parameters,
		max_correction_extent,
	}
}
