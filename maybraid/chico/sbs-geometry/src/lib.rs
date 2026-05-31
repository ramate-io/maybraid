//! Stalk and ball-stick geometry for Chico vegetation.
pub mod vec3_args;
pub use vec3_args::parse_vec3_csv;

pub mod chain;

pub use chain::date_palm::{DatePalmChain, DatePalmPhase};
pub use chain::liams_conifer::{LiamsConiferChain, LiamsConiferPhase};
pub use chain::sopes_banyan::{SopesBanyanChain, SopesBanyanPhase};
pub use chain::{
	BallStickChain, BallStickNode, BallStickSegment, BranchOut, DepthBudget, Hysteresis,
};

pub mod anchors;

pub use anchors::date_palm::{DatePalmAnchors, DatePalmProtoAnchors};
pub use anchors::liams_conifer::{LiamsConiferAnchors, LiamsConiferProtoAnchors};
pub use anchors::sopes_banyan::{SopesBanyanAnchors, SopesBanyanProtoAnchors};
pub use anchors::stalk_perturbation::{
	AnchorPerturbation, HasStrictStalk, PerturbAnchor, StalkPerturbation,
};
pub use anchors::strict_stalk::StrictStalk;
pub use anchors::{Anchors, AnchorsToChain};
pub use sbs::date_palm::DatePalmSbs;
pub use sbs::liams_conifer::LiamsConiferSbs;
pub use sbs::sopes_banyan::SopesBanyanSbs;

#[cfg(feature = "render")]
pub mod render;

pub mod sbs;
