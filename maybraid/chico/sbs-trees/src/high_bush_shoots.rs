//! **High Bush Shoots** — multi-shoot bush as [`VegetationComponents`].
//!
//! [`HighBushShootsParams::build`] grows [`HighBushShootsShape::build_chain`] once into
//! [`HighBushShoots`]. Sticks emit per segment (High all / Medium subsample / Low sparse);
//! foliage follows [`HighBushFoliageStyle`] (default layered-ball; cheap-ball at High/Medium
//! with a layered-ball Low proxy) using the Common High Bush ball-selection rule
//! (terminals, upper canopy, or branch order > 1) — not plane-splay.
//!
//! Legacy [`chico_tree_components::HighBushShoots`] RenderItem still uses
//! [`HighBushFoliageStyle::PlaneSplay`] / Tuft via ball-components.

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, HighBushChain};
use chico_tree_components::{
	should_allocate_foliage, HighBushFoliageStyle, HighBushShootsShape,
};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, Placement, StickNode,
	VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

/// High when `distance / footprint_radius ≤` this.
const HIGH_BUSH_STRUCTURAL_HIGH_FACTOR: f32 = 3.0;
const HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR: f32 = 12.0;
const HIGH_BUSH_STRUCTURAL_LOW_FACTOR: f32 = 24.0;

/// Authoring / CLI parameters for High Bush Shoots (VegetationComponents path).
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct HighBushShootsParams {
	#[command(flatten, next_help_heading = "High Bush Shape")]
	pub shape: HighBushShootsShape,
}

impl Default for HighBushShootsParams {
	fn default() -> Self {
		Self {
			shape: HighBushShootsShape {
				// VC path prefers layered-ball terminals; RenderItem keeps PlaneSplay default.
				foliage_style: HighBushFoliageStyle::LayeredBall,
				..HighBushShootsShape::default()
			},
		}
	}
}

impl HighBushShootsParams {
	pub fn new(shape: HighBushShootsShape) -> Self {
		Self { shape }
	}

	pub fn build(&self) -> HighBushShoots {
		HighBushShoots::from_params(self)
	}
}

/// Built high bush: shape plus grown ball-stick chain.
#[derive(Clone)]
pub struct HighBushShoots {
	pub shape: HighBushShootsShape,
	pub chain: BallStickChain<HighBushChain>,
}

impl HighBushShoots {
	pub fn from_params(params: &HighBushShootsParams) -> Self {
		Self {
			shape: params.shape.clone(),
			chain: params.shape.build_chain(),
		}
	}

	fn leaf_radius_world(&self) -> f32 {
		self.shape.leaf_radius_world()
	}

	fn footprint_radius(&self) -> f32 {
		self.chain.footprint_radius_at_least(
			(self.shape.height * self.shape.segment_radius_fraction_hi).max(1e-3),
		)
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.shape.height * 0.5, 0.0)
	}

	/// Stick stride: High=1 (all), Medium=2, Low=4.
	fn stick_nodes(&self, stride: usize) -> Vec<StickNode> {
		let stride = stride.max(1);
		self.chain
			.segments()
			.enumerate()
			.filter(|(i, _)| i % stride == 0)
			.filter_map(|(_, seg)| {
				StickNode::from_segment(seg.start.position, seg.end.position, seg.start.radius)
			})
			.collect()
	}

	/// Foliage at RFC ball-selection joints; `stride` thins candidates.
	///
	/// [`HighBushFoliageStyle::LayeredBall`] is layered at every LOD. Cheap / plane-splay /
	/// tuft stay cheap-ball at High/Medium and use a layered-ball Low proxy.
	fn foliage_nodes(&self, stride: usize, low: bool) -> Vec<FoliageNode> {
		let stride = stride.max(1);
		let leaf_r = self.leaf_radius_world().max(1e-4);
		let sites: Vec<Vec3> = self
			.chain
			.nodes_with_hysteresis_enumerated()
			.filter(|(idx, _, hyst)| should_allocate_foliage(*idx, hyst, &self.chain))
			.map(|(_, node, _)| node.position)
			.collect();
		let layered = matches!(self.shape.foliage_style, HighBushFoliageStyle::LayeredBall) || low;
		sites
			.into_iter()
			.enumerate()
			.filter(|(i, _)| i % stride == 0)
			.map(|(_, position)| {
				let placement = Placement::foliage_uniform(position, leaf_r);
				if layered {
					FoliageNode::layered_ball(placement)
				} else {
					FoliageNode::cheap_ball(placement)
				}
			})
			.collect()
	}
}

impl VegetationComponents for HighBushShoots {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let stride = match level {
			LodSceneLevel::High => 1,
			LodSceneLevel::Medium => 2,
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => 4,
		};
		Layers::from_free(self.stick_nodes(stride))
			.map(|n| n.with_material(chico_stick_material_ref()))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let (stride, low) = match level {
			LodSceneLevel::High => (1, false),
			LodSceneLevel::Medium => (2, false),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => (3, true),
		};
		// PlaneSplay / CheapBall / LayeredBall / Tuft all map to ball kits on the VC path.
		Layers::from_free(self.foliage_nodes(stride, low))
			.map(|n| n.with_material(chico_leaf_material_ref()))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		let radius = self
			.footprint_radius()
			.max(self.shape.height * 0.5)
			.max(1e-3);
		Some(
			StructuralLod::new(self.structural_center(), radius).with_factors(
				HIGH_BUSH_STRUCTURAL_HIGH_FACTOR,
				HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR,
				HIGH_BUSH_STRUCTURAL_LOW_FACTOR,
			),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_vc_uses_layered_ball_style() -> Result<()> {
		assert_eq!(
			HighBushShootsParams::default().shape.foliage_style,
			HighBushFoliageStyle::LayeredBall
		);
		Ok(())
	}

	#[test]
	fn high_emits_sticks_and_canopy_layered_balls() -> Result<()> {
		let built = HighBushShootsParams::default().build();
		let sticks = built.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!sticks.is_empty());
		let foliage = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!foliage.is_empty());
		// RFC ball selection fills more than graph terminals alone.
		let terminal_only = built
			.chain
			.nodes_with_hysteresis_enumerated()
			.filter(|(idx, _, _)| {
				chico_sbs_geometry::high_bush_is_graph_terminal(&built.chain, *idx)
			})
			.count();
		assert!(foliage.len() >= terminal_only);
		assert!(foliage.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}

	#[test]
	fn medium_subsamples_sticks_low_uses_layered() -> Result<()> {
		let built = HighBushShootsParams::default().build();
		let high = built.stick_nodes_for_level(LodSceneLevel::High).flatten();
		let medium = built.stick_nodes_for_level(LodSceneLevel::Medium).flatten();
		let low_sticks = built.stick_nodes_for_level(LodSceneLevel::Low).flatten();
		assert!(medium.len() <= high.len());
		assert!(low_sticks.len() <= medium.len());

		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert!(!low.is_empty());
		assert!(low.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}

	#[test]
	fn cheap_ball_style_emits_cheap_at_high() -> Result<()> {
		let mut params = HighBushShootsParams::default();
		params.shape.foliage_style = HighBushFoliageStyle::CheapBall;
		let built = params.build();
		let high = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(high.iter().all(|n| matches!(
			n.geometry,
			chico_vegetation_components::FoliageGeometry::CheapBall
		)));
		Ok(())
	}

	#[test]
	fn structural_lod_from_footprint() -> Result<()> {
		let built = HighBushShootsParams::default().build();
		let probe = built.structural_lod().expect("probe");
		assert!(probe.tree_radius > 0.0);
		Ok(())
	}
}
