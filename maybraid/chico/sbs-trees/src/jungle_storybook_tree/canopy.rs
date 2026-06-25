//! Jungle Storybook foliage allocation ([#235](https://github.com/ramate-io/maybraid/issues/235)).
//!
//! One [`BallRenderRule`] drives [`JungleStorybookCanopyFoliage`] per graph node:
//! - **Growth** — epiphyte clusters on mid/upper limbs (stochastic fraction).
//! - **InnerBall** — dense inner canopy (lower ring-u, non-terminal limbs).
//! - **OuterSplay** — terminal tips and far-along projections (RFC outer shell).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::chain::storybook_tree::{
	is_graph_terminal, StorybookTreeChain, StorybookTreePhase,
};
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::{mix_seed_below_fraction, node_mix_seed};
use chico_sbs_geometry::{BallStickChain, BallStickNode};
use chico_tree_components::{JungleGrowth, JungleGrowthShape, JungleStorybookCanopyFoliage};
use procedural_common::NoiseParams;

/// Spawn uniform scale for [`JungleStorybookCanopyFoliage::Growth`] relative to branch node radius.
pub const JUNGLE_GROWTH_RADIUS_SCALE: f32 = 2.0;

/// Minimum ring parameter `u` before a node may receive jungle growth (keeps trunk base clear).
const MIN_RING_U_FOR_GROWTH: f32 = 0.28;

/// Minimum ring `u` for any foliage when not terminal / high branch-order (RFC §3.1.7.13 inner fill).
const MIN_RING_U_FOR_FOLIAGE: f32 = 0.40;

/// Along a limb, fraction of [`StorybookTreeChain::projection_length`] past which outer splay is preferred.
const OUTER_SPLAY_DISTANCE_FRACTION: f32 = 0.55;

const RACHIS_THICKNESS_CENTER: f32 = 0.02;
const RACHIS_THICKNESS_SPAN: f32 = 0.004;
const FOLIAGE_SCALE_CENTER: f32 = 5.0;
const FOLIAGE_SCALE_SPAN: f32 = 1.4;
const GROWTH_FROND_COUNT: u32 = 8;
const RADIUS_SCALE_SPAN: f32 = 0.30;

/// Deterministic per-node unit interval for jitter lanes.
fn mix_unit(node_idx: usize, position: Vec3, lane: u32) -> f32 {
	(node_mix_seed(node_idx, position).wrapping_add(lane) as f32) / (u32::MAX as f32)
}

fn jitter(center: f32, span: f32, t: f32) -> f32 {
	(center + (t - 0.5) * span).max(1e-4)
}

/// Which foliage mesh (if any) to spawn at this branch-out node.
enum NodeFoliageKind {
	None,
	Growth,
	InnerBall,
	OuterSplay,
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
