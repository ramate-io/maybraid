//! Wraps inner [`Hysteresis`] with a **remaining** segment budget for canopy-style growth.

use crate::BallStickNode;

use super::Hysteresis;

/// [`Hysteresis`] plus how many canopy segments remain before flair / terminal transitions.
#[derive(Clone, Debug, PartialEq)]
pub struct DepthBudget<H> {
	pub inner: H,
	pub remaining: usize,
}

impl<H: Hysteresis> Hysteresis for DepthBudget<H> {
	fn ball_stick_node(&self) -> BallStickNode {
		self.inner.ball_stick_node()
	}

	fn next_hysteresis(&self) -> Vec<Self> {
		Vec::new()
	}
}
