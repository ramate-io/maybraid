//! Jungle Storybook foliage allocation ([#235](https://github.com/ramate-io/maybraid/issues/235)).
//!
//! VegetationComponents + legacy [`BallRenderRule`]:
//! - **Growth** — epiphyte clusters (palm fronds + spears under VC).
//! - **InnerBall** — dense inner canopy.
//! - **OuterSplay** — terminal tips and far-along projections.

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::chain::storybook_tree::{
	is_graph_terminal, StorybookTreeChain, StorybookTreePhase,
};
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::{mix_seed_below_fraction, node_mix_seed};
use chico_sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	BallStickNode,
};
use chico_tree_components::{JungleGrowth, JungleGrowthShape, JungleStorybookCanopyFoliage};
use chico_vegetation_components::{FoliageNode, Placement};
use procedural_common::NoiseParams;

use crate::jungle_growth_vc::{jungle_growth_foliage_nodes, JungleGrowthVcParams};
use crate::storybook_tree::canopy::HIGH_FOLIAGE_BANDS;

/// Growth spawn uniform-scale center (~4/5 of legacy RenderItem `2.0`).
pub const JUNGLE_GROWTH_RADIUS_SCALE: f32 = 1.6;

/// Medium / Low foliage sample grids (High reuses storybook [`HIGH_FOLIAGE_BANDS`] for canopy).
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);
/// Medium / Low growth bands (High emits every growth candidate).
pub(crate) const MEDIUM_GROWTH_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);
/// Minimum ring parameter `u` before a node may receive jungle growth (keeps trunk base clear).
const MIN_RING_U_FOR_GROWTH: f32 = 0.28;

/// Minimum ring `u` for any foliage when not terminal / high branch-order (RFC §3.1.7.13 inner fill).
const MIN_RING_U_FOR_FOLIAGE: f32 = 0.40;

/// Along a limb, fraction of [`StorybookTreeChain::projection_length`] past which outer splay is preferred.
const OUTER_SPLAY_DISTANCE_FRACTION: f32 = 0.55;

const RACHIS_THICKNESS_CENTER: f32 = 0.02;
const RACHIS_THICKNESS_SPAN: f32 = 0.004;
const FOLIAGE_SCALE_CENTER: f32 = 1.6;
const FOLIAGE_SCALE_SPAN: f32 = 0.40;
const GROWTH_FROND_COUNT: u32 = 8;
const RADIUS_SCALE_SPAN: f32 = 0.24;
const FULL_CANOPY_PROXY_RADIUS_SCALE: f32 = 0.70;

/// Deterministic per-node unit interval for jitter lanes.
fn mix_unit(node_idx: usize, position: Vec3, lane: u32) -> f32 {
	(node_mix_seed(node_idx, position).wrapping_add(lane) as f32) / (u32::MAX as f32)
}

fn jitter(center: f32, span: f32, t: f32) -> f32 {
	(center + (t - 0.5) * span).max(1e-4)
}

/// Which foliage mesh (if any) to spawn at this branch-out node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeFoliageKind {
	None,
	Growth,
	InnerBall,
	OuterSplay,
}

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	kind: NodeFoliageKind,
	node_idx: usize,
	leaf_radius: f32,
}

/// Branch-out nodes with non-zero projection that participate in the canopy pass.
fn qualifies_for_foliage(
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
	node_idx: usize,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	hysteresis.ring_u > MIN_RING_U_FOR_FOLIAGE
		|| is_graph_terminal(chain, node_idx)
		|| hysteresis.branch_order() > 1
}

fn classify_node_foliage(
	growth_spawn_fraction: f32,
	node_idx: usize,
	node: &BallStickNode,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
) -> NodeFoliageKind {
	if !qualifies_for_foliage(hysteresis, chain, node_idx) {
		return NodeFoliageKind::None;
	}
	if hysteresis.ring_u >= MIN_RING_U_FOR_GROWTH
		&& mix_seed_below_fraction(node_idx, node.position, growth_spawn_fraction)
	{
		return NodeFoliageKind::Growth;
	}
	let outer = hysteresis.distance_from_anchor
		> OUTER_SPLAY_DISTANCE_FRACTION * hysteresis.projection_length.max(1e-6);
	if is_graph_terminal(chain, node_idx) || outer {
		NodeFoliageKind::OuterSplay
	} else {
		NodeFoliageKind::InnerBall
	}
}

fn world_leaf_radius(node: &BallStickNode, leaf_radius_world: f32) -> f32 {
	let scale = leaf_radius_world / node.radius.max(1e-4);
	node.radius * scale
}

fn growth_params(node_idx: usize, position: Vec3) -> JungleGrowthVcParams {
	JungleGrowthVcParams::from_node(
		node_idx,
		position,
		JUNGLE_GROWTH_RADIUS_SCALE,
		RADIUS_SCALE_SPAN,
		FOLIAGE_SCALE_CENTER,
		FOLIAGE_SCALE_SPAN,
	)
}

fn emit_canopy_ball(_kind: NodeFoliageKind, position: Vec3, leaf_radius: f32) -> FoliageNode {
	FoliageNode::cheap_ball(Placement::foliage_uniform(position, leaf_radius))
}

fn collect_candidates(
	chain: &BallStickChain<StorybookTreeChain>,
	growth_spawn_fraction: f32,
	leaf_radius_world: f32,
) -> (Vec<FoliageCandidate>, Vec<FoliageCandidate>) {
	let mut growth = Vec::new();
	let mut canopy = Vec::new();
	for (node_idx, node, h) in chain.nodes_with_hysteresis_enumerated() {
		match classify_node_foliage(growth_spawn_fraction, node_idx, node, h, chain) {
			NodeFoliageKind::None => {}
			NodeFoliageKind::Growth => growth.push(FoliageCandidate {
				position: node.position,
				kind: NodeFoliageKind::Growth,
				node_idx,
				leaf_radius: world_leaf_radius(node, leaf_radius_world),
			}),
			kind => canopy.push(FoliageCandidate {
				position: node.position,
				kind,
				node_idx,
				leaf_radius: world_leaf_radius(node, leaf_radius_world),
			}),
		}
	}
	(growth, canopy)
}

fn banded_candidates(
	candidates: &[FoliageCandidate],
	bands: AzimuthHeightBands,
) -> Vec<&FoliageCandidate> {
	sample_max_horizontal_radius_by_azimuth_height(candidates, |c| c.position, bands)
		.into_iter()
		.map(|s| s.item)
		.collect()
}

fn full_canopy_proxy(
	growth: &[FoliageCandidate],
	canopy: &[FoliageCandidate],
) -> Option<FoliageNode> {
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for c in growth.iter().chain(canopy.iter()) {
		let r = c.leaf_radius;
		min = min.min(c.position - Vec3::splat(r));
		max = max.max(c.position + Vec3::splat(r));
		any = true;
	}
	if !any {
		return None;
	}
	let center = (min + max) * 0.5;
	let mut half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	half_extents.x *= FULL_CANOPY_PROXY_RADIUS_SCALE;
	half_extents.z *= FULL_CANOPY_PROXY_RADIUS_SCALE;
	Some(FoliageNode::layered_ball(
		Placement::new(center, 0.0).with_scale(half_extents),
	))
}

/// High: every jungle-growth site + banded inner/outer canopy.
pub(crate) fn foliage_nodes_high(
	chain: &BallStickChain<StorybookTreeChain>,
	growth_spawn_fraction: f32,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let (growth, canopy) = collect_candidates(chain, growth_spawn_fraction, leaf_radius_world);
	let mut nodes = Vec::new();
	for c in &growth {
		nodes.extend(jungle_growth_foliage_nodes(growth_params(c.node_idx, c.position)));
	}
	for c in banded_candidates(&canopy, HIGH_FOLIAGE_BANDS) {
		nodes.push(emit_canopy_ball(c.kind, c.position, c.leaf_radius));
	}
	nodes
}

/// Medium: banded growth (fronds only) + banded cheap canopy (no mass proxy).
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<StorybookTreeChain>,
	growth_spawn_fraction: f32,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let (growth, canopy) = collect_candidates(chain, growth_spawn_fraction, leaf_radius_world);
	let mut nodes = Vec::new();
	for c in banded_candidates(&growth, MEDIUM_GROWTH_BANDS) {
		nodes.extend(jungle_growth_foliage_nodes(growth_params(c.node_idx, c.position)));
	}
	for c in banded_candidates(&canopy, MEDIUM_FOLIAGE_BANDS) {
		nodes.push(emit_canopy_ball(c.kind, c.position, c.leaf_radius));
	}
	nodes
}

/// Low: cheap canopy + full-canopy layered proxy (no growth balls).
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	growth_spawn_fraction: f32,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let (growth, canopy) = collect_candidates(chain, growth_spawn_fraction, leaf_radius_world);
	let mut nodes = Vec::new();
	for c in banded_candidates(&canopy, LOW_FOLIAGE_BANDS) {
		nodes.push(emit_canopy_ball(c.kind, c.position, c.leaf_radius));
	}
	if let Some(proxy) = full_canopy_proxy(&growth, &canopy) {
		nodes.push(proxy);
	}
	nodes
}

#[allow(dead_code)]
fn build_jungle_growth<BodyM, BodyS, FoliageM, FoliageS>(
	node_idx: usize,
	node: &BallStickNode,
	body_noise: NoiseParams,
	foliage_noise: NoiseParams,
	body_material: BodyS,
	foliage_material: FoliageS,
) -> (JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>, f32)
where
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Default,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Default,
{
	let mut shape = JungleGrowthShape::default();
	shape.seed = (node_idx as i32)
		.wrapping_add(node.position.x.to_bits() as i32)
		.wrapping_add(node.position.y.to_bits().rotate_left(5) as i32);

	let pos = node.position;
	shape.frond.rachis_half_thickness =
		jitter(RACHIS_THICKNESS_CENTER, RACHIS_THICKNESS_SPAN, mix_unit(node_idx, pos, 11));
	shape.foliage_scale =
		jitter(FOLIAGE_SCALE_CENTER, FOLIAGE_SCALE_SPAN, mix_unit(node_idx, pos, 19));
	shape.frond.frond_count = GROWTH_FROND_COUNT;

	let radius_scale =
		jitter(JUNGLE_GROWTH_RADIUS_SCALE, RADIUS_SCALE_SPAN, mix_unit(node_idx, pos, 37));

	let mut growth = JungleGrowth::<BodyM, BodyS, FoliageM, FoliageS>::default();
	growth.shape = shape;
	growth.body_noise = body_noise;
	growth.foliage_noise = foliage_noise;
	growth.body_material = body_material;
	growth.foliage_material = foliage_material;
	(growth, radius_scale)
}

/// [`BallRenderRule`] for the unified jungle canopy enum (inner ball, outer splay, growth).
#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct JungleStorybookFoliageRule<
	InnerM,
	InnerS,
	OuterM,
	OuterS,
	BodyM,
	BodyS,
	FoliageM,
	FoliageS,
> where
	InnerM: Material,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>>,
	OuterM: Material,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>>,
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>>,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>>,
{
	/// Fraction of qualifying nodes that spawn [`JungleStorybookCanopyFoliage::Growth`] instead of canopy meshes.
	pub growth_spawn_fraction: f32,
	pub inner_ball: ChicoBall<InnerM, InnerS>,
	pub outer_splay: PlaneSplay<OuterM, OuterS>,
	/// Target world leaf radius from SBS [`leaf_radius_world`](chico_sbs_geometry::StorybookTreeSbs::leaf_radius_world).
	pub leaf_radius_world: f32,
	pub body_noise: NoiseParams,
	pub foliage_noise: NoiseParams,
	pub body_material: BodyS,
	pub foliage_material: FoliageS,
	pub(crate) __marker: PhantomData<fn() -> (BodyM, FoliageM)>,
}

#[allow(dead_code)]
impl<InnerM, InnerS, OuterM, OuterS, BodyM, BodyS, FoliageM, FoliageS>
	BallRenderRule<
		JungleStorybookCanopyFoliage<
			InnerM,
			InnerS,
			OuterM,
			OuterS,
			BodyM,
			BodyS,
			FoliageM,
			FoliageS,
		>,
		StorybookTreeChain,
	> for JungleStorybookFoliageRule<InnerM, InnerS, OuterM, OuterS, BodyM, BodyS, FoliageM, FoliageS>
where
	InnerM: Material + Send + Sync + 'static,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>> + Send + Sync + 'static,
	OuterM: Material + Send + Sync + 'static,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>> + Send + Sync + 'static,
	BodyM: Material + Send + Sync + 'static,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Send + Sync + 'static + Default,
	FoliageM: Material + Send + Sync + 'static,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Send + Sync + 'static + Default,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &StorybookTreeChain,
		chain: &BallStickChain<StorybookTreeChain>,
	) -> Option<(
		JungleStorybookCanopyFoliage<
			InnerM,
			InnerS,
			OuterM,
			OuterS,
			BodyM,
			BodyS,
			FoliageM,
			FoliageS,
		>,
		f32,
	)> {
		let scale = self.leaf_radius_world / node.radius.max(1e-4);
		match classify_node_foliage(self.growth_spawn_fraction, node_idx, node, hysteresis, chain) {
			NodeFoliageKind::None => None,
			NodeFoliageKind::Growth => {
				let (growth, radius_scale) = build_jungle_growth(
					node_idx,
					node,
					self.body_noise,
					self.foliage_noise,
					self.body_material.clone(),
					self.foliage_material.clone(),
				);
				Some((JungleStorybookCanopyFoliage::Growth(growth), radius_scale))
			}
			NodeFoliageKind::InnerBall => {
				Some((JungleStorybookCanopyFoliage::InnerBall(self.inner_ball.clone()), scale))
			}
			NodeFoliageKind::OuterSplay => {
				let mut splay = self.outer_splay.clone();
				let seed = node_mix_seed(node_idx, node.position);
				splay.icosphere_subdivisions = seed % 2;
				splay.leaf_disc_radius = 0.18 + 0.12 * ((seed % 17) as f32 / 16.0);
				Some((JungleStorybookCanopyFoliage::OuterSplay(splay), scale))
			}
		}
	}
}
