//! Assemble stream corridor depression + shared apron from a laid-out path.

use crate::apron::{jittered_depth, ApronNoiseSalts};
use crate::complex::WatershedDepressionComplex;
use crate::depression::{WatershedDepression, WatershedDepressionKind};
use crate::hydro::{primitives_from_polyline, ComplexApronParams, DEFAULT_RIM_UPLIFT_CAP};
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

/// Stream-specific stamp: one corridor as hydro primitives + apron params.
///
/// Convert with [`Self::into_complex`] → [`WatershedDepressionComplex`].
#[derive(Debug, Clone)]
pub(crate) struct StreamCorridor {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub half_width: f32,
	pub wet_core: Region2D,
	pub center_depth: f32,
	pub hydro_apron: ComplexApronParams,
}

impl StreamCorridor {
	/// `StreamCorridor` → sole-edge [`WatershedDepressionComplex`].
	pub fn into_complex(self, bounds: Bounds2, seed: u32) -> WatershedDepressionComplex {
		let influence = self.hydro_apron.rim_width + self.hydro_apron.apron_width;
		let primitives = primitives_from_polyline(
			&self.path,
			&self.levels,
			self.half_width,
			self.center_depth,
			influence,
		);
		let mut complex = WatershedDepressionComplex::new(bounds, seed);
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
		complex.with_hydro(primitives, self.hydro_apron)
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
	let rim_w = (skirt_w * 0.35).max(2.0).min(half_w);
	let apron_width = (apron_w - skirt_w).max(apron_band);

	StreamCorridor {
		path,
		levels,
		half_width: half_w,
		wet_core: channel_region,
		center_depth,
		hydro_apron: ComplexApronParams {
			rim_lift: params.rim_lift.max(0.0),
			rim_width: rim_w,
			apron_width,
			rim_height: apron_noise.rim_height,
			rim_uplift_cap: DEFAULT_RIM_UPLIFT_CAP,
			shore_fade: params.shore_fade.max(0.25),
			fill_undercut: params.fill_undercut.max(0.0),
		},
	}
}
