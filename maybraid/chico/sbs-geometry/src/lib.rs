//! Stalk and ball-stick geometry for Chico vegetation.
pub mod projection;
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
	segment_fracs, stalk_tip_from_chain, storybook_branch_depth, StorybookTreeChain, StorybookTreePhase,
	STORYBOOK_BRANCH_DEPTH_MAX, STORYBOOK_BRANCH_DEPTH_MIN,
};
pub use chain::penmarch_torch::{
	is_graph_terminal as penmarch_is_graph_terminal, PenmarchTorchChain,
};
pub use chain::kamakura_torch::{
	is_graph_terminal as kamakura_is_graph_terminal, KamakuraTorchChain,
};
pub use chain::rorys_head_trained::{
	is_graph_terminal as rorys_head_trained_is_graph_terminal, RorysHeadTrainedChain,
};
pub use chain::vase_tree::{is_graph_terminal as vase_tree_is_graph_terminal, VaseTreeChain};
pub use chain::liams_conifer::{
	liams_conifer_branch_depth, stalk_tip_from_chain as liams_stalk_tip_from_chain, LiamsConiferChain,
	LiamsConiferPhase, SEGMENT_FRACS,
};
pub use chain::sopes_banyan::{SopesBanyanChain, SopesBanyanPhase};
pub use chain::honu_banyan::{is_graph_terminal as honu_banyan_is_graph_terminal, HonuBanyanChain, HonuBanyanPhase};
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
pub use anchors::friends_conifer::{
	FriendsConiferAnchors, FriendsConiferChain, FriendsConiferProtoAnchors,
};
pub use projection::{
	logarithmic_rounding_projection, vase_profile, vase_projection_length,
};
pub use anchors::penmarch_torch::{
	penmarch_torch_branch_direction, PenmarchTorchAnchors, PenmarchTorchProtoAnchors,
};
pub use anchors::kamakura_torch::{
	kamakura_torch_branch_direction, KamakuraTorchAnchors, KamakuraTorchProtoAnchors,
};
pub use anchors::rorys_head_trained::{
	rorys_flat_projection_length, rorys_head_trained_branch_direction, RorysHeadTrainedAnchors,
	RorysHeadTrainedProtoAnchors,
};
pub use anchors::vase_tree::{
	vase_tree_branch_direction, VaseTreeAnchors, VaseTreeProtoAnchors,
	DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
};
pub use anchors::sopes_banyan::{SopesBanyanAnchors, SopesBanyanProtoAnchors};
pub use anchors::honu_banyan::{
	honu_canopy_bias, honu_projection_length, HonuBanyanAnchors, HonuBanyanProtoAnchors,
};
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
pub use sbs::northern_conifer::NorthernConiferSbs;
pub use sbs::friends_conifer::FriendsConiferSbs;
pub use sbs::sopes_banyan::SopesBanyanSbs;
pub use sbs::honu_banyan::HonuBanyanSbs;
pub use sbs::penmarch_torch::PenmarchTorchSbs;
pub use sbs::kamakura_torch::KamakuraTorchSbs;
pub use sbs::rorys_head_trained::RorysHeadTrainedSbs;
pub use sbs::vase_tree::VaseTreeSbs;

#[cfg(feature = "render")]
pub mod render;

pub mod sbs;
