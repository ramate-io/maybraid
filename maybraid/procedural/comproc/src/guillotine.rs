//! Bounded **guillotine** partitions of axis-aligned hyper-rectangles.
//!
//! Layout rule inspired by
//! [RFC-127 §3.1.2 Pocket Cells](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#312-pocket-cells),
//! implemented as **middle-out max-fitting** through-cuts on the root:
//!
//! 1. Choose an axis that is not fully saturated (both outward fronts stuck).
//! 2. Choose a low or high front; sample a step in `[step_min, step_max]`; place the cut
//!    outward from the axis midpoint.
//! 3. If the candidate would leave the root, discard it and saturate that front.
//!
//! This is greedy packing: end remainders may be smaller than `step_min`. Leaf regions are
//! the cartesian product of per-axis intervals. Noise queries use the root lower-left plus
//! attempt / channel salts via [`crate::noise::config::NoiseConfig`] (decorrelation only).
//!
//! Modules:
//! - [`bounds`] — `Bounds<D>` and 1–4D aliases
//! - [`config`] — step window / snap
//! - [`cutter`] — fixed-depth [`Guillotine`]
//! - [`regions`] — cut lists + region iterators
//! - [`variable`] — depth-range [`VariableGuillotine`]

pub mod bounds;
pub mod config;
pub mod cutter;
pub mod regions;
pub mod variable;

pub use bounds::{Bounds, Bounds1, Bounds2, Bounds3, Bounds4};
pub use config::GuillotineConfig;
pub use cutter::{Guillotine, RegionsIntoIter, RegionsOwned};
pub use regions::{GuillotineCuts, Regions};
pub use variable::{DepthRange, VariableGuillotine};

#[cfg(test)]
mod tests;
