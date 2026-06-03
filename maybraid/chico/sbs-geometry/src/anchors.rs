//! Ball-stick **anchor** recipes: emit seed hysteresis values for [`crate::BallStickChain::build`].
//!
//! # Single graph (stalk + canopy)
//!
//! We do **not** split stalk and canopy into separate builders. The trunk is part of the **same** ball-stick graph as the crown: a vertical run of nodes with **strict** (zero–DOF) hysteresis forms the stalk; **ring** seeds at the stalk **radial centroid** start canopy chains. [`Anchors::anchors`] emits one [`Hysteresis`] value per seed (carrying the root [`BallStickNode`] and growth parameters).
//!
//! Downstream [`Hysteresis`] implementations (e.g. Sope's Banyan) then grow branches from those seeds via [`Hysteresis::next_hysteresis`].

pub mod date_palm;
pub mod waialea_palm;
pub mod storybook_tree;
pub mod braid_oak;
pub mod liams_conifer;
pub mod friends_conifer;
pub mod sopes_banyan;
pub mod torch_tree;
pub mod penmarch_torch;
pub mod kamakura_torch;
pub mod stalk_perturbation;
pub mod strict_stalk;

use crate::{BallStickChain, Hysteresis};

/// Produces seed hysteresis for one [`BallStickChain::build`] call (one state per chain arm).
pub trait Anchors<T: Hysteresis> {
	fn anchors(&self) -> Vec<T>;
}

/// Common workflow for builders that emit hysteresis seeds for one ball-stick chain.
pub trait AnchorsToChain<T: Hysteresis>: Anchors<T> {
	fn build_chain(&self) -> BallStickChain<T> {
		BallStickChain::build(self.anchors())
	}

	fn chains(&self) -> Vec<BallStickChain<T>> {
		vec![self.build_chain()]
	}
}

impl<A, T> AnchorsToChain<T> for A
where
	A: Anchors<T>,
	T: Hysteresis,
{
}
