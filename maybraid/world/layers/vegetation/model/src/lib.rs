//! Vegetation as a layer over a [`VegetationModel`].
//!
//! [`Vegetation<V>`] is `V::Ground` with the same heights. Storage stays in the
//! model crate; this crate only declares the contract.

mod generation;
mod model;
mod vegetation;

pub use generation::{VegetationGeneration, VegetationGenerationSystems};
pub use model::Vegetation;
pub use vegetation::VegetationModel;

#[cfg(test)]
mod tests;
