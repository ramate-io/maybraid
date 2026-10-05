//! Language as a layer over a [`LanguageModel`].
//!
//! [`Language<L>`] is a sibling of furnishing over the vegetated world. Nothing
//! wraps it, and it does not change the ground. Storage stays in the model crate.

mod generation;
mod language;
mod model;

pub use generation::{LanguageGeneration, LanguageGenerationSystems};
pub use language::LanguageModel;
pub use model::Language;

#[cfg(test)]
mod tests;
