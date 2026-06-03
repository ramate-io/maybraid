//! Stalk and ball-stick geometry for Chico vegetation.
pub mod vec3_args;
pub use vec3_args::parse_vec3_csv;

pub mod chain;

pub use chain::arch_trunk::{
	arch_horizontal_direction_from_yaw_degrees, arch_point, arch_point_from_params,
	normalize_arch_horizontal_direction, ArchTrunk, ArchTrunkParams,
};
pub use chain::date_palm::{DatePalmChain, DatePalmPhase};
pub use chain::waialea_palm::{WaialeaPalmChain, WaialeaPalmPhase};
pub use chain::storybook_tree::{
	segment_fracs, storybook_branch_depth, StorybookTreeChain, StorybookTreePhase,
	STORYBOOK_BRANCH_DEPTH_MAX, STORYBOOK_BRANCH_DEPTH_MIN,
};
pub use chain::liams_conifer::{LiamsConiferChain, LiamsConiferPhase};
pub use chain::sopes_banyan::{SopesBanyanChain, SopesBanyanPhase};
pub use chain::{
	BallStickChain, BallStickNode, BallStickSegment, BranchOut, DepthBudget, Hysteresis,
};

pub mod anchors;

pub use anchors::date_palm::{DatePalmAnchors, DatePalmProtoAnchors};
pub use anchors::waialea_palm::{WaialeaPalmAnchors, WaialeaPalmProtoAnchors};
pub use anchors::storybook_tree::{
	dome_projection_length, StorybookTreeAnchors, StorybookTreeProtoAnchors,
};
pub use anchors::braid_oak::{
	braid_vertical_bias_radial, BraidOakTreeAnchors, BraidOakTreeProtoAnchors,
};
pub use anchors::liams_conifer::{LiamsConiferAnchors, LiamsConiferProtoAnchors};
pub use anchors::sopes_banyan::{SopesBanyanAnchors, SopesBanyanProtoAnchors};
pub use anchors::stalk_perturbation::{
	AnchorPerturbation, HasStrictStalk, PerturbAnchor, StalkPerturbation,
};
pub use anchors::strict_stalk::StrictStalk;
pub use anchors::{Anchors, AnchorsToChain};
pub use sbs::date_palm::DatePalmSbs;
pub use sbs::waialea_palm::WaialeaPalmSbs;
pub use sbs::storybook_tree::StorybookTreeSbs;
pub use sbs::jungle_storybook_tree::JungleStorybookTreeSbs;
pub use sbs::braid_oak_tree::BraidOakTreeSbs;
pub use sbs::liams_conifer::LiamsConiferSbs;
pub use sbs::sopes_banyan::SopesBanyanSbs;

#[cfg(feature = "render")]
pub mod render;

pub mod sbs;
