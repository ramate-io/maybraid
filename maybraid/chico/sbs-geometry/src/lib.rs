//! Stalk and ball-stick geometry for Chico vegetation.
pub mod chain;

pub use chain::{
	BallStickChain, BallStickNode, BallStickSegment, ChainHysteresisRule, Hysteresis,
	PeriodicHysteresisRule,
};

#[cfg(feature = "render")]
pub mod render;
