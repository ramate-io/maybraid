//! Species definitions.
//!
//! A species owns its baseline silhouette, supported assets, defaults, and the
//! mapping from resolved controls to rig/feature effects.

pub mod braidman;
pub mod brenal;
pub mod caole;
pub mod claber;
pub mod croconot;
pub mod hars;
pub mod brodler;
pub mod common;
pub mod dui;
pub mod lero;
pub mod lidder;
pub mod mygr;
pub mod spibmom;
pub mod wumbus;
pub mod ylter;
pub mod sonyak;

use crate::ResolvedCharacterAssembly;

/// Type-owned resolution contract for species-specific configs.
pub trait SpeciesConfig {
	fn species_name(&self) -> &'static str;

	fn resolve(&self) -> ResolvedCharacterAssembly;
}
