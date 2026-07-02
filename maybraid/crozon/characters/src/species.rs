//! Species definitions.
//!
//! A species owns its baseline silhouette, supported assets, defaults, and the
//! mapping from resolved controls to rig/feature effects.

pub mod braidman;
pub mod brodler;
pub mod common;
pub mod mygr;

use crate::ResolvedCharacterAssembly;

/// Type-owned resolution contract for species-specific configs.
pub trait SpeciesConfig {
	fn species_name(&self) -> &'static str;

	fn resolve(&self) -> ResolvedCharacterAssembly;
}
