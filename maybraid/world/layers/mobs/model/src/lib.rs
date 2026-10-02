//! Mobs as a layer over a [`MobModel`].
//!
//! [`Mobs<B>`] is `B::Ground` with the same heights. Storage stays in the model
//! crate; this crate only declares the contract.

mod generation;
mod mob;
mod model;

pub use generation::{
	MobGeneration, MobGenerationCore, MobGenerationPlugin, MobGenerationSystems, MobModeConfig,
	MobScheme,
};
pub use mob::{MobCellPresented, MobModel};
pub use model::Mobs;

#[cfg(test)]
mod tests;
