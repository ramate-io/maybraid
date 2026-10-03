//! Furnishing as a layer over a [`FurnishingModel`].
//!
//! [`Furnishing<F>`] is a sibling of vegetation over urbanization. Nothing wraps
//! it, and it does not change the ground. Storage stays in the model crate.

mod furnishing;
mod generation;
mod model;

pub use furnishing::FurnishingModel;
pub use generation::{
	FurnishingGeneration, FurnishingGenerationCore, FurnishingGenerationPlugin,
	FurnishingGenerationSystems, FurnishingModeConfig, FurnishingScheme,
};
pub use model::Furnishing;

#[cfg(test)]
mod tests;
