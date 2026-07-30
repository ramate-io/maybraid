//! Species definitions.
//!
//! A species owns its baseline silhouette, supported assets, defaults, and the
//! mapping from resolved controls to rig/feature effects.

pub mod braidman;
pub mod brenal;
pub mod brodler;
pub mod brokker;
pub mod caole;
pub mod chupri;
pub mod claber;
pub mod common;
pub mod croconot;
pub mod dui;
pub mod epiphant;
pub mod grener;
pub mod hars;
pub mod kaller;
pub mod kappler;
pub mod kispar;
pub mod lero;
pub mod lidder;
pub mod mistler;
pub mod mygr;
pub mod sonyak;
pub mod spibmom;
pub mod tapp;
pub mod thumplus;
pub mod tipple;
pub mod topple;
pub mod tuberwaber;
pub mod wumbus;
pub mod ylter;

use crate::ResolvedCharacterAssembly;

/// Type-owned resolution contract for species-specific configs.
pub trait SpeciesConfig {
	fn species_name(&self) -> &'static str;

	fn resolve(&self) -> ResolvedCharacterAssembly;
}
