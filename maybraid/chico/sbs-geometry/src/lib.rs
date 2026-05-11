//! Stalk and ball-stick geometry for Chico vegetation.
pub mod chain;

pub use chain::sopes_banyan::{SopesBanyanChain, SopesBanyanHysteresis, SopesBanyanPhase};
pub use chain::{
	BallStickChain, BallStickNode, BallStickSegment, BranchOut, DepthBudget, Hysteresis,
};

pub mod anchors;

pub use anchors::sopes_banyan::SopesBanyanAnchors;
pub use anchors::strict_stalk::StrictStalk;

#[cfg(feature = "render")]
pub mod render;

pub mod sbs;
