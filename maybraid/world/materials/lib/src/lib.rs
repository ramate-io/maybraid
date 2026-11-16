//! Composed Maybraid World [`material_ref::MaterialLib`].
//!
//! Domain shaders stay in their crates until a larger grouping exists. The
//! mythical chain lives here as [`chain`] because it does not yet have one.
//! This crate is the global recipe-name table: furniture, character, urban,
//! muzzle flame, energy, mythical chain, then vegetation and Standard.

pub mod chain;
mod material_lib;

pub use material_lib::{
	WorldMaterialLib, WorldMaterialRefPlugin, WorldMaterialsPlugin, WorldMaterialsShadersPlugin,
};
