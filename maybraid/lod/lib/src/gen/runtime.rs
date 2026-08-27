//! Generate-region production and budgeted index insert.
//!
//! These plugins do not add the scene refresh stack. They share only
//! [`crate::lod_ref::LodNodePlugin`]. Present plugins live in
//! [`crate::presentation`].

mod generate;

#[cfg(test)]
mod tests;

pub use generate::{
	drain_lod_generate, produce_lod_generate_regions, LodGenerateBudget, LodGenerateKeepRegion,
	LodGeneratePlugin, LodGenerateQueue, LodGenerateRegion, LodGenerateRegionPlugin,
	LodGenerateSystems,
};
