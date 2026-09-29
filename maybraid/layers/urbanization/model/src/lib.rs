//! Urbanization as a terrain model: `Urbanization<M>` is model `M` with
//! development pads composed in, plus the urban artifacts built on it.
//!
//! Storage stays in Richmond's [`DevelopmentEntryStore`](richmond_development_models::DevelopmentEntryStore)
//! and [`UrbanizationIndex`](richmond_urbanization::UrbanizationIndex); this
//! crate only declares the contract over them. [`UrbanizationGenerationPlugin`]
//! lives here because it is what brings `Urbanization<M>` into existence.

mod generation;
mod model;
mod pads;
mod urban;

pub use generation::UrbanizationGenerationPlugin;
pub use model::{UrbanRead, UrbanSnapshot, Urbanization};
pub use pads::PadComposable;
pub use urban::UrbanModel;

#[cfg(test)]
mod tests;
