//! Streams graph — multi-corridor stream leaf for composition practice.
//!
//! Grows a degree-bounded [`HysteresisGraph`], collapses chains between
//! keypoints into corridors, then emits one [`WatershedDepressionComplex`]
//! through [`crate::compose::StreamBandComposer`] (soft-voronoi apron blend,
//! owned thalweg/rim, one \(W\) field). No lake/bog decoration yet.

use crate::apron::{ApronNoiseSalts, WatershedApronParams};
use crate::complex::{WatershedDepressionComplex, WatershedEdge, WatershedNode};
use crate::compose::{StreamBandComposer, StreamBandPart, DEFAULT_RIM_UPLIFT_CAP};
use crate::depression::{WatershedDepression, WatershedDepressionKind};
use crate::noise::n01_freq;
use crate::stream::{
	collapse_degenerate_vertices, node_water_levels, resolve_node_blend, sample_endpoint,
	StreamBandBudget, StreamParams, DEGENERATE_VERTEX_EPS, ENDPOINT_A_SALT, ENDPOINT_B_SALT,
};
use bevy_math::Vec2;
use jersey_terrain_stamps::{DownhillPair, PolylineRegion, Region2D};
use procedural_common::{Bounds2, HysteresisGraph, SeededHash};

/// Minimum channel half-width (world units); smaller budgets skip the stamp.
const MIN_HALF_WIDTH: f32 = 3.0;
/// Minimum path length (world units) along a corridor.
const MIN_PATH_LEN: f32 = 20.0;

const WIDTH_SCALE_SALT: u32 = 0x57EA_512E;
const DEGREE_SALT: u32 = 0x57EA_D6EE;

/// Shared rim budget for stream graphs (not solo 15–120 m defaults).
const GRAPH_RIM_AMP_MIN: f32 = 0.0;
const GRAPH_RIM_AMP_MAX: f32 = 1.25;
/// Extra thalweg cut on top of the freeboard bed grade. Solo streams use full
/// `StreamParams::depth` (6–10 m); graphs already absolute-grade the channel, so
/// stacking another full depth digs canyons that glue to leftover ridges.
const GRAPH_THALWEG_DEPTH_FRAC: f32 = 0.3;
const GRAPH_THALWEG_DEPTH_MAX: f32 = 2.5;
const GRAPH_DEPTH_NOISE_FRAC: f32 = 0.35;

/// Authoring knobs for a streams-graph leaf.
#[derive(Debug, Clone, Copy)]
pub struct StreamsGraphParams {
	pub stream: StreamParams,
	/// Inclusive min hysteresis out-degree (`2..=4` typical).
	pub degree_min: u8,
	/// Inclusive max hysteresis out-degree.
	pub degree_max: u8,
	/// Hard cap on shared add-only rim height noise.
	pub rim_uplift_cap: f32,
}

impl Default for StreamsGraphParams {
	fn default() -> Self {
		let mut stream = StreamParams::default();
		// Fill ⊆ carve for graph composition (autopsy water-wall fix).
		stream.fill_half_width_scale = 1.0;
		stream.shore_fade = 2.5;
		stream.fill_undercut = 2.0;
		stream.apron = muted_graph_apron_params(stream.apron);
		Self {
			stream,
			degree_min: 2,
			degree_max: 3,
			rim_uplift_cap: DEFAULT_RIM_UPLIFT_CAP,
		}
	}
}

fn muted_graph_apron_params(mut apron: WatershedApronParams) -> WatershedApronParams {
	apron.rim_height_amp_min = GRAPH_RIM_AMP_MIN;
	apron.rim_height_amp_max = GRAPH_RIM_AMP_MAX;
	apron
}

fn width_scale_u01(seed: u32, leaf_min: Vec2, params: StreamParams) -> f32 {
	n01_freq(seed, WIDTH_SCALE_SALT, leaf_min, params.half_width_freq)
}

/// Pin corridor endpoints to the **local** keypoint sample so every incident
/// corridor agrees on junction \(W\), without `min`-dragging uphill heads down
/// to a foreign valley grade.
fn pin_junction_water_levels(
	corridors: &mut [CorridorPart],
	key_points: &[Vec2],
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
	sink: f32,
) {
	let sink = sink.max(0.0);
	if key_points.is_empty() || corridors.is_empty() {
		return;
	}
	let key_w: Vec<f32> = key_points
		.iter()
		.map(|p| height_at.map(|f| f(p.x, p.y)).unwrap_or(0.0) - sink)
		.collect();
	let n_keys = key_w.len();
	for c in corridors.iter_mut() {
		let n = c.band.levels.len();
		if n < 2 || c.from_key >= n_keys || c.to_key >= n_keys {
			continue;
		}
		let head = key_w[c.from_key];
		let toe = key_w[c.to_key].min(head);
		c.band.levels[0] = head;
		c.band.levels[n - 1] = toe;
		for i in 1..n {
			c.band.levels[i] = c.band.levels[i].min(c.band.levels[i - 1]);
		}
		c.band.levels[n - 1] = toe;
		for i in (1..n - 1).rev() {
			c.band.levels[i] = c.band.levels[i]
				.min(c.band.levels[i - 1])
				.max(c.band.levels[n - 1]);
		}
	}
}

/// One collapsed corridor between keypoints (composer input + graph edge).
#[derive(Debug, Clone)]
struct CorridorPart {
	from_key: usize,
	to_key: usize,
	band: StreamBandPart,
	wet_core: Region2D,
}

/// Authored streams-graph **plan**: hysteresis corridors → one composed complex.
#[derive(Debug, Clone)]
pub struct StreamsGraph {
	pub bounds: Bounds2,
	pub seed: u32,
	pub degree: u8,
	pub half_width: f32,
	pub edge_count: usize,
	corridors: Vec<CorridorPart>,
	key_points: Vec<Vec2>,
	composer: StreamBandComposer,
}

impl StreamsGraph {
	/// Build a multi-corridor stream plan, or `None` when the leaf / graph is too small.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: StreamsGraphParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Option<Self> {
		let min = bounds.min;
		let max = bounds.max;
		let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
		let short_half = half.x.min(half.y).max(1.0);

		let stream_p = params.stream;
		let width_u = width_scale_u01(seed, min, stream_p);
		let budget = StreamBandBudget::try_from_short_half(short_half, stream_p, width_u)?;
		if budget.half_width < MIN_HALF_WIDTH {
			return None;
		}

		let inset = budget.apron_half.max(budget.mu);
		let lo = min + Vec2::splat(inset);
		let hi = max - Vec2::splat(inset);
		if lo.x >= hi.x || lo.y >= hi.y {
			return None;
		}

		let a0 = sample_endpoint(seed, ENDPOINT_A_SALT, lo, hi);
		let b0 = sample_endpoint(seed, ENDPOINT_B_SALT, lo, hi);
		if a0.distance(b0) < MIN_PATH_LEN * 0.5 {
			return None;
		}
		let (head_xz, _, toe_xz, _) = DownhillPair::order(a0, b0, height_at);

		let d_lo = params.degree_min.max(2).min(4);
		let d_hi = params.degree_max.max(d_lo).min(4);
		let degree = if d_lo == d_hi {
			d_lo
		} else {
			let span = (d_hi - d_lo + 1) as f32;
			let u = SeededHash::new(seed.wrapping_add(DEGREE_SALT)).unit(0);
			(d_lo + (u * span).floor() as u8).min(d_hi)
		};

		let walk = stream_p.spine.walk_config(bounds);
		let graph = HysteresisGraph::with_degree(
			degree,
			bounds,
			seed.wrapping_add(21),
			head_xz,
			toe_xz,
			&walk,
		);
		if graph.nodes.len() < 3 {
			return None;
		}

		let node_blend = resolve_node_blend(stream_p, bounds, budget.half_width);
		// Channel already absolute-grades to \(W - \mathrm{freeboard}\); keep the
		// extra thalweg nick shallow so min-compose overlaps cannot canyon.
		let depth = (stream_p.depth.max(0.0) * GRAPH_THALWEG_DEPTH_FRAC)
			.min(GRAPH_THALWEG_DEPTH_MAX)
			.max(0.35);
		let depth_noise_amp =
			(stream_p.depth_noise_amp.max(0.0) * GRAPH_DEPTH_NOISE_FRAC).min(0.75);
		let apron_band = (budget.apron_half - budget.skirt_half).max(0.5);
		let apron_noise = stream_p.apron.sample_noise(
			seed,
			min,
			apron_band,
			budget.half_width,
			ApronNoiseSalts::STREAM,
		);

		let n = graph.nodes.len();
		let mut is_key = vec![false; n];
		for i in 0..n {
			let out = graph.children.get(i).map(|c| c.len()).unwrap_or(0);
			is_key[i] = i == 0 || out != 1;
		}

		let mut key_graph_idx = Vec::new();
		for i in 0..n {
			if is_key[i] {
				key_graph_idx.push(i);
			}
		}
		if key_graph_idx.len() < 2 {
			return None;
		}

		let sample_w = |p: Vec2| {
			height_at.map(|f| f(p.x, p.y)).unwrap_or(0.0) - stream_p.water_sink.max(0.0)
		};

		let mut corridors = Vec::new();
		for &from in &key_graph_idx {
			for &child in graph.children.get(from).into_iter().flatten() {
				let mut path_idx = vec![from, child];
				let mut cur = child;
				let mut ok = true;
				while !is_key[cur] {
					let Some(&next) = graph.children.get(cur).and_then(|c| c.first()) else {
						ok = false;
						break;
					};
					path_idx.push(next);
					cur = next;
				}
				if !ok || path_idx.len() < 2 {
					continue;
				}
				let mut path: Vec<Vec2> = path_idx.iter().map(|&k| graph.nodes[k]).collect();
				let Some(mut from_key) = key_graph_idx.iter().position(|&k| k == from) else {
					continue;
				};
				let Some(mut to_key) = key_graph_idx.iter().position(|&k| k == cur) else {
					continue;
				};
				// Grade each corridor on its own polyline (never flatten-clamp the
				// whole `graph.nodes` vec — spur tips were inheriting unrelated
				// valley \(W\) and carving ridges into hydrological nonsense).
				if sample_w(*path.last().expect("path")) > sample_w(path[0]) {
					path.reverse();
					std::mem::swap(&mut from_key, &mut to_key);
				}
				let mut levels =
					node_water_levels(&path, height_at, stream_p.water_sink, stream_p.min_drop);
				collapse_degenerate_vertices(&mut path, &mut levels, DEGENERATE_VERTEX_EPS);
				if path.len() < 2 {
					continue;
				}
				let path_len: f32 = path.windows(2).map(|w| w[0].distance(w[1])).sum();
				if path_len < MIN_PATH_LEN * 0.35 {
					continue;
				}

				let shore_amp = (budget.half_width * stream_p.shore_indent_frac.clamp(0.0, 0.4))
					.max(0.01);
				let shore_freq = stream_p
					.shore_freq
					.max(1.5 / budget.half_width.max(1.0))
					.clamp(1.0e-4, 0.14);
				let shore_noise = jersey_terrain_stamps::RegionNoise::from_seed(
					seed.wrapping_add(5).wrapping_add(corridors.len() as u32),
					shore_freq,
					shore_amp,
				);
				let depth_noise = jersey_terrain_stamps::RegionNoise::from_seed(
					seed.wrapping_add(9).wrapping_add(corridors.len() as u32),
					(1.4 / budget.half_width.max(1.0)).clamp(0.04, 0.2),
					depth_noise_amp,
				);

				let wet_core =
					Region2D::Polyline(PolylineRegion::new(path.clone(), budget.half_width));
				corridors.push(CorridorPart {
					from_key,
					to_key,
					band: StreamBandPart {
						path,
						levels,
						half_width: budget.half_width,
						thalweg_half: budget.thalweg_half,
						skirt_half: budget.skirt_half,
						apron_half: budget.apron_half,
						node_blend,
						freeboard: stream_p.channel_freeboard.max(0.25),
						rim_lift: stream_p.rim_lift.max(0.0),
						depth,
						shore_indent_noise: Some(shore_noise),
						depth_noise: Some(depth_noise),
						apron_boundary_noise: Some(apron_noise.apron.clone()),
					},
					wet_core,
				});
			}
		}
		if corridors.is_empty() {
			return None;
		}

		let key_points = key_graph_idx
			.iter()
			.map(|&i| graph.nodes[i])
			.collect::<Vec<_>>();

		// Shared junction \(W\) from local keypoint samples (not min-of-corridors).
		pin_junction_water_levels(
			&mut corridors,
			&key_points,
			height_at,
			stream_p.water_sink,
		);

		let band_parts: Vec<StreamBandPart> = corridors.iter().map(|c| c.band.clone()).collect();
		let composer = StreamBandComposer::new(band_parts, apron_noise.rim_height)
			.with_rim_uplift_cap(params.rim_uplift_cap.min(DEFAULT_RIM_UPLIFT_CAP))
			.with_fill_undercut(stream_p.fill_undercut.max(0.0))
			.with_shore_fade(stream_p.shore_fade.max(0.25));

		Some(Self {
			bounds,
			seed,
			degree,
			half_width: budget.half_width,
			edge_count: corridors.len(),
			corridors,
			key_points,
			composer,
		})
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Option<Self> {
		Self::from_bounds(bounds, seed, StreamsGraphParams::default(), None)
	}

	/// Realize as a multi-edge complex with soft-voronoi stream-band composition.
	pub fn into_complex(self) -> WatershedDepressionComplex {
		let mut complex = WatershedDepressionComplex::new(self.bounds, self.seed);
		let mut node_ids = Vec::with_capacity(self.key_points.len());
		for _ in &self.key_points {
			node_ids.push(complex.push_node(WatershedNode::empty()));
		}
		for corridor in &self.corridors {
			let from = node_ids[corridor.from_key];
			let to = node_ids[corridor.to_key];
			complex.push_edge(WatershedEdge {
				from,
				to,
				depression: WatershedDepression::new(
					WatershedDepressionKind::StreamCorridor,
					corridor.wet_core.clone(),
					Vec::new(),
					None,
				),
			});
		}
		complex.with_stream_bands(self.composer)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fill::WaterSurface;

	fn slope_height(x: f32, z: f32) -> f32 {
		100.0 - 0.05 * x - 0.02 * z
	}

	#[test]
	fn leaf_too_small_skips() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 30.0, 30.0);
		assert!(StreamsGraph::from_bounds_default(bounds, 11).is_none());
		Ok(())
	}

	#[test]
	fn builds_multi_edge_composed_complex() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let g = StreamsGraph::from_bounds(
			bounds,
			42,
			StreamsGraphParams::default(),
			Some(&slope_height),
		)
		.expect("streams graph");
		assert!(g.edge_count >= 1);
		let compiled = g.into_complex().compile();
		assert_eq!(compiled.modulations.len(), 3);
		assert_eq!(compiled.fills.len(), 1);
		assert!(matches!(
			compiled.fills[0].surface,
			WaterSurface::OwnedGraded { .. }
		));
		Ok(())
	}

	#[test]
	fn junction_water_is_continuous() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let g = StreamsGraph::from_bounds(
			bounds,
			7,
			StreamsGraphParams::default(),
			Some(&slope_height),
		)
		.expect("streams graph");
		let compiled = g.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		// Sample near each keypoint — owned W should be finite and locally smooth.
		for p in &g.key_points {
			let w0 = fill.surface_level_at(p.x, p.y);
			let w1 = fill.surface_level_at(p.x + 0.5, p.y);
			assert!(
				(w0 - w1).abs() < 2.0,
				"W jump near junction {} → {}: {w0} vs {w1}",
				p,
				Vec2::new(p.x + 0.5, p.y)
			);
		}
		Ok(())
	}

	#[test]
	fn freeboard_under_wet_softmask() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let params = StreamsGraphParams::default();
		let freeboard = params.stream.channel_freeboard;
		let g = StreamsGraph::from_bounds(bounds, 19, params, Some(&slope_height)).expect("graph");
		let compiled = g.clone().into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		for corridor in &g.corridors {
			assert!(corridor.band.path.len() >= 2);
			for window in corridor.band.path.windows(2) {
				let mid = window[0].lerp(window[1], 0.5);
				if fill.softmask_at(mid.x, mid.y) > 0.85 {
					continue;
				}
				let w = fill.surface_level_at(mid.x, mid.y);
				let mut h = slope_height(mid.x, mid.y);
				for m in &compiled.modulations {
					h = m.modify_elevation(h, mid.x, mid.y);
				}
				assert!(
					h <= w - freeboard * 0.35,
					"bed {h} should sit under W {w} (freeboard {freeboard}) at {mid:?}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn overlap_does_not_pillar_above_rim_cap() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let mut params = StreamsGraphParams::default();
		params.rim_uplift_cap = 1.5;
		let g = StreamsGraph::from_bounds(bounds, 3, params, Some(&slope_height)).expect("graph");
		let compiled = g.into_complex().compile();
		let mut max_rise = 0.0f32;
		for i in 0..40 {
			for j in 0..40 {
				let x = bounds.min.x + (i as f32 + 0.5) * (bounds.max.x - bounds.min.x) / 40.0;
				let z = bounds.min.y + (j as f32 + 0.5) * (bounds.max.y - bounds.min.y) / 40.0;
				let h0 = slope_height(x, z);
				let mut h = h0;
				for m in &compiled.modulations {
					h = m.modify_elevation(h, x, z);
				}
				max_rise = max_rise.max((h - h0).max(0.0));
			}
		}
		// Bank lift (~rim_lift) + capped rim noise; stacked solo aprons would be >> 20 m.
		assert!(
			max_rise < 12.0,
			"composed graph should not pillar: max_rise={max_rise}"
		);
		Ok(())
	}

	#[test]
	fn corridor_w_tracks_local_head_not_foreign_valley() -> anyhow::Result<()> {
		// Rough terrain: the old flat-array `node_water_levels(&graph.nodes)` clamp
		// dragged spur tips down to unrelated valley \(W\).
		let height = |x: f32, z: f32| 80.0 + 25.0 * (x * 0.02).sin() + 18.0 * (z * 0.03).cos();
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let params = StreamsGraphParams::default();
		let sink = params.stream.water_sink;
		let freeboard = params.stream.channel_freeboard;
		let thalweg = (params.stream.depth * GRAPH_THALWEG_DEPTH_FRAC)
			.min(GRAPH_THALWEG_DEPTH_MAX);
		let g = StreamsGraph::from_bounds(bounds, 11, params, Some(&height)).expect("graph");
		let compiled = g.clone().into_complex().compile();
		for corridor in &g.corridors {
			let head = corridor.band.path[0];
			let head_w = corridor.band.levels[0];
			let head_local = height(head.x, head.y) - sink;
			assert!(
				(head_local - head_w).abs() < 0.75,
				"head W {head_w} should match local sample {head_local} at {head:?}"
			);
			for (p, &w) in corridor
				.band
				.path
				.iter()
				.zip(corridor.band.levels.iter())
			{
				let mut h = height(p.x, p.y);
				for m in &compiled.modulations {
					h = m.modify_elevation(h, p.x, p.y);
				}
				assert!(
					h >= w - freeboard - thalweg - 3.0,
					"bed {h} far below corridor W {w} at {p:?} (over-deep thalweg / bad W)"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn fill_half_width_not_liberal() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 500.0, 500.0);
		let g = StreamsGraph::from_bounds(
			bounds,
			42,
			StreamsGraphParams::default(),
			Some(&slope_height),
		)
		.expect("graph");
		let half = g.half_width;
		let compiled = g.into_complex().compile();
		let fill = compiled.fills.first().expect("fill");
		// Spot-check: a point just outside channel half-width should be mostly dry.
		if let Some(corridor) = compiled.wet_union.as_ref() {
			let c = corridor.center();
			let far = Vec2::new(c.x, c.y + half * 2.5);
			assert!(
				fill.softmask_at(far.x, far.y) > 0.5,
				"fill should not extend far past channel carve"
			);
		}
		Ok(())
	}
}
