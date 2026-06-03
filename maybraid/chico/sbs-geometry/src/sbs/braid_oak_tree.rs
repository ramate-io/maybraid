//! **Braid Oak Tree** SBS frontend ([#234](https://github.com/ramate-io/maybraid/issues/234)).
//!
//! Wraps [`StorybookTreeSbs`] with braid art-direction defaults. Flattened clap still exposes storybook
//! field defaults; call [`BraidOakTreeSbs::apply_braid_preset`] after CLI parse.

use std::ops::{Deref, DerefMut};

use procedural_common::{NoiseParams, SetNoiseParams, UnitRange};

use crate::anchors::braid_oak::{
	BraidOakTreeAnchors, BraidOakTreeProtoAnchors, BRAID_ANCHORS_PER_RING_MAX,
	BRAID_ANGLE_TOLERANCE_DEGREES, BRAID_BRANCH_BASE_RADIUS_FRACTION_OF_STALK, BRAID_BRANCH_DEPTH,
	BRAID_FIRST_RING_UNIT_HEIGHT, BRAID_LEAF_RADIUS_FRACTION, BRAID_MAX_PROJECTION_FRACTION,
	BRAID_PROJECTION_END_FRACTION, BRAID_RING_SPACING_UNIT_HEIGHT, BRAID_STALK_BASE_RADIUS_FRACTION,
	BRAID_STALK_HEIGHT_FRACTION,
};
use crate::anchors::storybook_tree::DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION;
use crate::anchors::{Anchors, AnchorsToChain};
use crate::sbs::storybook_tree::{
	StorybookCanopyParams, StorybookGrowthParams, StorybookProjectionParams,
	StorybookRingParams, StorybookTreeSbs, StorybookTreeScale,
};
use crate::{BallStickChain, StorybookTreeChain};

fn braid_oak_fields(h: f32) -> StorybookTreeSbs {
	StorybookTreeSbs {
		scale: StorybookTreeScale {
			tree_height: h,
			stalk_height_fraction: BRAID_STALK_HEIGHT_FRACTION,
			stalk_base_radius: Some(BRAID_STALK_BASE_RADIUS_FRACTION * h),
			..StorybookTreeScale::default()
		},
		rings: StorybookRingParams {
			height_range: UnitRange::new(BRAID_FIRST_RING_UNIT_HEIGHT, 1.0),
			spacing: BRAID_RING_SPACING_UNIT_HEIGHT,
			anchors_per_ring: BRAID_ANCHORS_PER_RING_MAX,
		},
		projection: StorybookProjectionParams {
			max_projection_fraction: BRAID_MAX_PROJECTION_FRACTION,
			projection_end_fraction: BRAID_PROJECTION_END_FRACTION,
		},
		growth: StorybookGrowthParams {
			branch_depth: BRAID_BRANCH_DEPTH,
			angle_tolerance_degrees: BRAID_ANGLE_TOLERANCE_DEGREES,
			ring_tilt_degrees: 0.0,
			child_count_min: 2,
			child_count_max: 2,
			branch_base_radius_fraction_of_stalk: BRAID_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale_lo: 0.82,
			branch_radius_child_scale_hi: 0.90,
			..StorybookGrowthParams::default()
		},
		canopy: StorybookCanopyParams {
			leaf_radius_fraction: BRAID_LEAF_RADIUS_FRACTION,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			..StorybookCanopyParams::default()
		},
		..StorybookTreeSbs::default()
	}
}

/// Flattened [`StorybookTreeSbs`] with braid preset constants.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct BraidOakTreeSbs {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub storybook: StorybookTreeSbs,
}

impl Default for BraidOakTreeSbs {
	fn default() -> Self {
		Self {
			storybook: braid_oak_fields(crate::anchors::storybook_tree::DEFAULT_TREE_HEIGHT),
		}
	}
}

fn apply_if_storybook_default<T: Clone + PartialEq>(current: &mut T, story: &T, braid: &T) {
	if *current == *story {
		*current = braid.clone();
	}
}

fn apply_braid_ring_preset(
	current: &mut StorybookRingParams,
	story: &StorybookRingParams,
	braid: &StorybookRingParams,
) {
	if current.height_range == story.height_range {
		current.height_range = braid.height_range;
	} else {
		apply_if_storybook_default(
			&mut current.height_range.start,
			&story.height_range.start,
			&braid.height_range.start,
		);
		apply_if_storybook_default(
			&mut current.height_range.end,
			&story.height_range.end,
			&braid.height_range.end,
		);
	}
	apply_if_storybook_default(&mut current.spacing, &story.spacing, &braid.spacing);
	apply_if_storybook_default(
		&mut current.anchors_per_ring,
		&story.anchors_per_ring,
		&braid.anchors_per_ring,
	);
}

impl Deref for BraidOakTreeSbs {
	type Target = StorybookTreeSbs;

	fn deref(&self) -> &Self::Target {
		&self.storybook
	}
}

impl DerefMut for BraidOakTreeSbs {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.storybook
	}
}

impl BraidOakTreeSbs {
	/// Reapply braid preset after flattened clap parse.
	pub fn apply_braid_preset(&mut self) {
		let braid = Self::default().storybook;
		let story = StorybookTreeSbs::default();
		let s = &mut self.storybook;
		let h = s.scale.tree_height.max(1e-6);

		apply_if_storybook_default(
			&mut s.scale.stalk_height_fraction,
			&story.scale.stalk_height_fraction,
			&braid.scale.stalk_height_fraction,
		);

		let story_stalk = crate::anchors::storybook_tree::DEFAULT_STALK_BASE_RADIUS_FRACTION * h;
		let braid_stalk = BRAID_STALK_BASE_RADIUS_FRACTION * h;
		match s.scale.stalk_base_radius {
			None => s.scale.stalk_base_radius = Some(braid_stalk),
			Some(r) if (r - story_stalk).abs() < 1e-4 => {
				s.scale.stalk_base_radius = Some(braid_stalk);
			}
			_ => {}
		}

		apply_braid_ring_preset(&mut s.rings, &story.rings, &braid.rings);
		apply_if_storybook_default(&mut s.projection, &story.projection, &braid.projection);
		apply_if_storybook_default(&mut s.growth, &story.growth, &braid.growth);
		apply_if_storybook_default(&mut s.canopy, &story.canopy, &braid.canopy);
	}

	pub fn height(&self) -> f32 {
		self.storybook.height()
	}

	pub fn leaf_radius_world(&self) -> f32 {
		self.storybook.leaf_radius_world()
	}

	pub fn to_proto(&self) -> BraidOakTreeProtoAnchors {
		BraidOakTreeProtoAnchors {
			tree_height: self.height(),
			stalk: self.scale.to_stalk(),
			first_ring_unit_height: self.rings.height_range.start,
			last_ring_unit_height: self.rings.height_range.end,
			ring_spacing_unit_height: self.rings.spacing,
			anchors_per_ring: self.rings.anchors_per_ring,
			max_projection_fraction_of_height: self.projection.max_projection_fraction,
			projection_end_fraction: self.projection.projection_end_fraction,
			branch_angle_tolerance: self.growth.angle_tolerance_degrees.to_radians(),
			bias_blend: 0.88,
			branch_depth: self.growth.branch_depth,
			child_count_min: self.growth.child_count_min,
			child_count_max: self.growth.child_count_max.max(self.growth.child_count_min),
			outer_foliage_distance_fraction: self.canopy.outer_foliage_distance_fraction,
			branch_base_radius_fraction_of_stalk: self.growth.branch_base_radius_fraction_of_stalk,
			branch_radius_child_scale: (
				self.growth.branch_radius_child_scale_lo,
				self.growth.branch_radius_child_scale_hi,
			),
		}
	}

	pub fn to_anchors(&self) -> BraidOakTreeAnchors {
		BraidOakTreeAnchors::new(self.to_proto())
			.with_perturbation(self.anchor_perturbation.to_perturbation())
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		AnchorsToChain::build_chain(self)
	}
}

impl Anchors<StorybookTreeChain> for BraidOakTreeSbs {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		let noise = procedural_common::NoiseConfig::new(self.canopy_noise);
		self.to_anchors().hysteresis_seeds(noise)
	}
}

impl SetNoiseParams for BraidOakTreeSbs {
	fn with_noise_params(mut self, params: NoiseParams) -> Self {
		self.storybook = self.storybook.with_noise_params(params);
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_builds_chain() -> Result<()> {
		let chain = BraidOakTreeSbs::default().build_chain();
		assert!(chain.nodes.len() > 20, "nodes {}", chain.nodes.len());
		Ok(())
	}

	#[test]
	fn apply_braid_preset_after_storybook_cli_defaults() -> Result<()> {
		let mut geometry = BraidOakTreeSbs { storybook: StorybookTreeSbs::default() };
		geometry.apply_braid_preset();
		let proto = geometry.to_proto();
		assert!((proto.stalk.stalk_height - geometry.height() * BRAID_STALK_HEIGHT_FRACTION).abs() < 1e-3);
		assert!((proto.first_ring_unit_height - BRAID_FIRST_RING_UNIT_HEIGHT).abs() < 1e-4);
		assert_eq!(proto.child_count_min, 2);
		assert_eq!(proto.child_count_max, 2);
		assert_eq!(proto.branch_depth, BRAID_BRANCH_DEPTH);
		Ok(())
	}

	#[test]
	fn leaf_radius_scales_with_tree_height() -> Result<()> {
		let sbs = BraidOakTreeSbs::default();
		assert!((sbs.leaf_radius_world() - BRAID_LEAF_RADIUS_FRACTION * sbs.height()).abs() < 1e-4);
		Ok(())
	}
}
