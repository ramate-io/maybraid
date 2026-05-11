//! Ball-stick **anchor** recipes: emit seed hysteresis values for [`crate::BallStickChain::build`].
//!
//! # Single graph (stalk + canopy)
//!
//! We do **not** split stalk and canopy into separate builders. The trunk is part of the **same** ball-stick graph as the crown: a vertical run of nodes with **strict** (zero–DOF) hysteresis forms the stalk; **ring** seeds at the stalk **radial centroid** start canopy chains. [`Anchors::anchors`] emits one [`Hysteresis`] value per seed (carrying the root [`BallStickNode`] and growth parameters).
//!
//! Downstream [`Hysteresis`] implementations (e.g. Sope's Banyan) then grow branches from those seeds via [`Hysteresis::next_hysteresis`].

pub mod sopes_banyan;
pub mod strict_stalk;

use crate::Hysteresis;

/// Produces seed hysteresis for one [`BallStickChain::build`] call (one state per chain arm).
pub trait Anchors<T: Hysteresis> {
	fn anchors(&self) -> Vec<T>;
}
