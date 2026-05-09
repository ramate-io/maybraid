//! Stalk and ball-stick geometry for Chico vegetation.
pub mod chain;

pub use chain::{
	BallStickChain, BallStickNode, BallStickSegment, ChainHysteresisRule, Hysteresis,
	PeriodicHysteresisRule,
};

pub mod anchors;

#[cfg(feature = "render")]
pub mod render;
