//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! # Intent
//!
//! Wire the vertical **vase banyan** recipe: [Banyan Trunk §3.1.6.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/05-banyan-trunk/README.md) stalk (`chico_sdf` / stalk height and radius fractions from the RFC), `chico-sbs-geometry` anchor rings plus [`chico_sbs_geometry::chain::sopes_banyan`](chico_sbs_geometry::chain::sopes_banyan) hysteresis, segment meshes via `chico-stick` ([noisy tapered cylinder](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/README.md)), and canopy balls via `chico-ball` and `plane-splay` with RFC ball selection (foliage broadly in the rising crown; sparse on descenders unless tuning for denser mystique). Optional `tree-components` / jungle growth (tufts) comes later for dense variants per [§3.1.6.4 Jungle growths](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md).
//!
//! # Playground / CLI
//!
//! Everything that parameterizes height, rings, chain phases, materials, and optional fruiting ([§3.1.6.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/07-fruiting-bodies/README.md)) should be exposed **under feature flags** as **`clap`-parseable** types so a future playground can drive the same recipe as production.

use bevy::prelude::*;
use chico_ball_components::{chico_ball::ChicoBall, plane_splay::PlaneSplay};
use chico_sbs_geometry::anchors::Anchors;
use chico_sbs_geometry::render::ball::{BallRenderHelper, BallRenderRule};
use chico_sbs_geometry::render::stick::{StickRenderHelper, StickRenderRule};
use chico_sbs_geometry::{
	BallStickChain, BallStickNode, BallStickSegment, SopesBanyanAnchors, SopesBanyanChainRule,
	SopesBanyanHysteresis,
};
use chico_stick_components::chico_stick::ChicoStick;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};

#[derive(Clone)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct SopesBanyan {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub anchors: SopesBanyanAnchors,
	#[cfg_attr(feature = "clap", command(flatten))]
	pub chain_rule: SopesBanyanChainRule,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 0.0))]
	pub stick_seed_scalar: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub stick_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub stick_amplitude: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1))]
	pub stick_octaves: u32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub ball_frequency: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1.0))]
	pub ball_amplitude: f32,
	#[cfg_attr(feature = "clap", arg(long, default_value_t = 1))]
	pub ball_octaves: u32,
}

impl Default for SopesBanyan {
	fn default() -> Self {
		Self {
			anchors: SopesBanyanAnchors::default(),
			chain_rule: SopesBanyanChainRule::default(),
			stick_seed_scalar: 0.0,
			stick_frequency: 1.0,
			stick_amplitude: 0.05,
			stick_octaves: 1,
			ball_frequency: 1.0,
			ball_amplitude: 0.05,
			ball_octaves: 1,
		}
	}
}

impl SopesBanyan {
	pub fn build_chain(&self) -> BallStickChain<SopesBanyanHysteresis> {
		let starts = self.anchors.anchors();
		let mut rule = self.chain_rule.clone();
		rule.sync_noise_engine();
		BallStickChain::build(starts, &rule)
	}
}

#[derive(Clone)]
enum SopesBanyanBallItem {
	Chico(ChicoBall),
	Plane(PlaneSplay),
}

impl RenderItem for SopesBanyanBallItem {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		match self {
			Self::Chico(item) => item.spawn_render_items(commands, cascade_chunk, transform),
			Self::Plane(item) => item.spawn_render_items(commands, cascade_chunk, transform),
		}
	}
}

#[derive(Clone)]
struct SopesBanyanBallRule {
	frequency: f32,
	amplitude: f32,
	octaves: u32,
}

impl BallRenderRule<SopesBanyanBallItem, SopesBanyanHysteresis> for SopesBanyanBallRule {
	fn ball_render_item_for(
		&self,
		node: &BallStickNode,
		hysteresis: &SopesBanyanHysteresis,
	) -> Option<SopesBanyanBallItem> {
		// Sparse balls on strong descenders, richer allocation on rising crown.
		if hysteresis.bias_ray.y < -0.8 {
			return None;
		}
		// let seed = node.position.x * 13.0 + node.position.y * 7.0 + node.position.z * 5.0;
		let seed = 0.0;
		if node.position.y > 0.6 * hysteresis.max_depth as f32 {
			Some(SopesBanyanBallItem::Plane(PlaneSplay::from_scalar(
				seed,
				self.frequency,
				self.amplitude,
				self.octaves,
			)))
		} else {
			Some(SopesBanyanBallItem::Chico(ChicoBall::from_scalar(
				seed,
				self.frequency,
				self.amplitude,
				self.octaves,
			)))
		}
	}
}

#[derive(Clone)]
struct SopesBanyanStickRule {
	seed_scalar: f32,
	frequency: f32,
	amplitude: f32,
	octaves: u32,
}

impl StickRenderRule<ChicoStick, SopesBanyanHysteresis> for SopesBanyanStickRule {
	fn stick_render_item_for(
		&self,
		segment: &BallStickSegment<'_>,
		_parent_hysteresis: &SopesBanyanHysteresis,
		_child_hysteresis: &SopesBanyanHysteresis,
	) -> Option<ChicoStick> {
		let seed =
			self.seed_scalar + segment.start.position.length() + segment.end.position.length();
		Some(ChicoStick::from_scalar(seed, self.frequency, self.amplitude, self.octaves))
	}
}

impl RenderItem for SopesBanyan {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let chain = self.build_chain();

		let stick_rule = SopesBanyanStickRule {
			seed_scalar: self.stick_seed_scalar,
			frequency: self.stick_frequency,
			amplitude: self.stick_amplitude,
			octaves: self.stick_octaves,
		};
		let mut out = StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		);

		let ball_rule = SopesBanyanBallRule {
			frequency: self.ball_frequency,
			amplitude: self.ball_amplitude,
			octaves: self.ball_octaves,
		};
		out.extend(BallRenderHelper::new(chain, ball_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));
		out
	}
}
