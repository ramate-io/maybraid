//! Urbanization as a layer over a [`UrbanizationModel`].
//!
//! [`Urbanization<U>`] is `U::Ground` with pads composed in, plus the urban
//! artifacts `U` stores. Storage stays in the model crate; this crate only
//! declares the contract.

mod generation;
mod model;
mod pads;
mod region;
mod setting;
mod urban;

pub use generation::{
	UrbanizationGeneration, UrbanizationGenerationCore, UrbanizationGenerationPlugin,
	UrbanizationGenerationSystems, UrbanizationLayerRegion, UrbanizationModeConfig,
	UrbanizationScheme, UrbanizationStoreSystems,
};
pub use model::{UrbanRead, UrbanSnapshot, Urbanization};
pub use pads::PadOps;
pub use region::{urbanization_host_region, urbanization_visual_region};
pub use setting::UrbanSetting;
pub use urban::{UrbanModel, UrbanSource, UrbanizationModel};

#[cfg(test)]
mod tests;
