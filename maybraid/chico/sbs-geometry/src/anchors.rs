//! Ball-stick **anchor** recipes: emit seed `(node, hysteresis)` pairs for [`crate::BallStickChain::build`].
//!
//! # Single graph (stalk + canopy)
//!
//! We do **not** split stalk and canopy into separate builders. The trunk is part of the **same** ball-stick graph as the crown: a vertical run of nodes with **strict** (zero–DOF) hysteresis forms the stalk; **ring** seeds at the stalk **radial centroid** start canopy chains. [`Anchors::anchors`] is only responsible for **generating those points** (positions, radii, and initial [`Hysteresis`]) that become the `start_nodes` argument to [`BallStickChain::build`].
//!
//! Downstream [`crate::ChainHysteresisRule`] (e.g. Sope's Banyan) then grows branches from those seeds.

pub mod sopes_banyan;
pub mod strict_stalk;

use crate::{BallStickNode, Hysteresis};

/// Produces seed nodes for one [`BallStickChain::build`] call: each pair is a root position + hysteresis for that chain arm.
pub trait Anchors {
	fn anchors(&self) -> Vec<(BallStickNode, Hysteresis)>;
}
