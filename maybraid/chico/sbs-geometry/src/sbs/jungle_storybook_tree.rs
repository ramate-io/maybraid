//! **Jungle Storybook Tree** SBS frontend ([#235](https://github.com/ramate-io/maybraid/issues/235)) — same geometry as Storybook with wider-branching defaults.

use std::ops::{Deref, DerefMut};

use procedural_common::{NoiseParams, SetNoiseParams, UnitRange};

use crate::anchors::storybook_tree::{
	DEFAULT_FIRST_RING_UNIT_HEIGHT, DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
	DEFAULT_PROJECTION_END_FRACTION, DEFAULT_STALK_BASE_RADIUS_FRACTION,
	DEFAULT_STALK_HEIGHT_FRACTION, DEFAULT_TREE_HEIGHT,
};
use crate::anchors::Anchors;
use crate::sbs::storybook_tree::{
	StorybookCanopyParams, StorybookGrowthParams, StorybookProjectionParams, StorybookRingParams,
	StorybookTreeSbs, StorybookTreeScale,
};
use crate::{BallStickChain, StorybookTreeChain};

/// Jungle stalk base radius as a fraction of `H` (storybook [`DEFAULT_STALK_BASE_RADIUS_FRACTION`]).
///
/// Used by [`Default`] and [`JungleStorybookTreeSbs::apply_jungle_preset`], not by flattened clap defaults.
pub const JUNGLE_STALK_BASE_RADIUS_FRACTION: f32 = 0.055;

/// Limb girth at ring anchors relative to stalk base (storybook growth default `0.12`).
///
/// Used by [`Default`] and [`JungleStorybookTreeSbs::apply_jungle_preset`], not by flattened clap defaults.
pub const JUNGLE_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.30;

/// Jungle variant: flattens [`StorybookTreeSbs`] with denser rings and wider branch fan-out.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
#[cfg_attr(feature = "clap", command(rename_all = "kebab-case"))]
pub struct JungleStorybookTreeSbs {
	#[cfg_attr(feature = "clap", command(flatten))]
	pub storybook: StorybookTreeSbs,
}

impl Default for JungleStorybookTreeSbs {
	fn default() -> Self {
		let h = DEFAULT_TREE_HEIGHT;
		Self {
			storybook: StorybookTreeSbs {
				scale: StorybookTreeScale {
					tree_height: h,
					stalk_height_fraction: DEFAULT_STALK_HEIGHT_FRACTION,
					stalk_base_radius: Some(JUNGLE_STALK_BASE_RADIUS_FRACTION * h),
					..StorybookTreeScale::default()
				},
				rings: StorybookRingParams {
					height_range: UnitRange::new(DEFAULT_FIRST_RING_UNIT_HEIGHT, 1.0),
					spacing: 0.08,
					anchors_per_ring: 7,
				},
				projection: StorybookProjectionParams {
					max_projection_fraction: 0.58,
					projection_end_fraction: DEFAULT_PROJECTION_END_FRACTION,
				},
				growth: StorybookGrowthParams {
					branch_depth: 4,
					angle_tolerance_degrees: 33.0,
					ring_tilt_degrees: 4.0,
					child_count_min: 2,
					child_count_max: 3,
					branch_base_radius_fraction_of_stalk:
						JUNGLE_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
					branch_radius_child_scale_lo: 0.82,
					branch_radius_child_scale_hi: 0.90,
				},
				canopy: StorybookCanopyParams {
					leaf_radius_fraction: 0.09,
					outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
				},
				..StorybookTreeSbs::default()
			},
		}
	}
}

fn apply_if_storybook_default<T: Clone + PartialEq>(current: &mut T, story: &T, jungle: &T) {
	if *current == *story {
		*current = jungle.clone();
	}
}

impl Deref for JungleStorybookTreeSbs {
	type Target = StorybookTreeSbs;

	fn deref(&self) -> &Self::Target {
		&self.storybook
	}
}

impl DerefMut for JungleStorybookTreeSbs {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.storybook
	}
}

impl JungleStorybookTreeSbs {
	/// Reapply jungle art-direction on a flattened [`StorybookTreeSbs`] (e.g. after CLI parse).
	///
	/// Clap fills nested storybook fields with storybook defaults; this restores jungle preset values
	/// for fields that still match the storybook default so explicit CLI overrides are kept.
	pub fn apply_jungle_preset(&mut self) {
		let preset = Self::default();
		let story = StorybookTreeSbs::default();
		let s = &mut self.storybook;
		let p = &preset.storybook;
		let h = s.scale.tree_height.max(1e-6);

		let story_stalk = DEFAULT_STALK_BASE_RADIUS_FRACTION * h;
		let jungle_stalk = JUNGLE_STALK_BASE_RADIUS_FRACTION * h;
		match s.scale.stalk_base_radius {
			None => s.scale.stalk_base_radius = Some(jungle_stalk),
			Some(r) if (r - story_stalk).abs() < 1e-4 => {
				s.scale.stalk_base_radius = Some(jungle_stalk);
			}
			_ => {}
		}

		apply_if_storybook_default(&mut s.rings, &story.rings, &p.rings);
		apply_if_storybook_default(&mut s.projection, &story.projection, &p.projection);
		apply_if_storybook_default(&mut s.growth, &story.growth, &p.growth);
		apply_if_storybook_default(&mut s.canopy, &story.canopy, &p.canopy);
	}

	pub fn height(&self) -> f32 {
		self.storybook.height()
	}

	pub fn leaf_radius_world(&self) -> f32 {
		self.storybook.leaf_radius_world()
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		self.storybook.build_chain()
	}
}

impl Anchors<StorybookTreeChain> for JungleStorybookTreeSbs {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.storybook.anchors()
	}
}

impl SetNoiseParams for JungleStorybookTreeSbs {
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
		let chain = JungleStorybookTreeSbs::default().build_chain();
		assert!(chain.nodes.len() > 50, "nodes {}", chain.nodes.len());
		Ok(())
	}

	#[test]
	fn apply_jungle_preset_after_storybook_cli_defaults() -> Result<()> {
		let mut geometry = JungleStorybookTreeSbs { storybook: StorybookTreeSbs::default() };
		geometry.apply_jungle_preset();
		let proto = geometry.storybook.to_proto();
		let story_proto = StorybookTreeSbs::default().to_proto();
		assert!(proto.stalk.stalk_base_radius > story_proto.stalk.stalk_base_radius);
		assert!(
			proto.branch_base_radius_fraction_of_stalk
				> story_proto.branch_base_radius_fraction_of_stalk
		);
		assert!(proto.anchors_per_ring >= story_proto.anchors_per_ring);
		Ok(())
	}

	#[test]
	fn jungle_has_wider_branching_than_storybook() -> Result<()> {
		let jungle = JungleStorybookTreeSbs::default();
		let story = StorybookTreeSbs::default();
		let jungle_proto = jungle.storybook.to_proto();
		let story_proto = story.to_proto();
		assert!(jungle_proto.branch_angle_tolerance > story_proto.branch_angle_tolerance);
		assert!(jungle_proto.child_count_min >= story_proto.child_count_min);
		assert!(jungle_proto.anchors_per_ring >= story_proto.anchors_per_ring);
		assert!(
			jungle_proto.stalk.stalk_base_radius > story_proto.stalk.stalk_base_radius,
			"stalk {} vs {}",
			jungle_proto.stalk.stalk_base_radius,
			story_proto.stalk.stalk_base_radius
		);
		assert!(
			jungle_proto.branch_base_radius_fraction_of_stalk
				> story_proto.branch_base_radius_fraction_of_stalk
		);
		let jungle_nodes = jungle.build_chain().nodes.len();
		let story_nodes = story.build_chain().nodes.len();
		assert!(jungle_nodes >= story_nodes, "{jungle_nodes} vs {story_nodes}");
		Ok(())
	}
}
