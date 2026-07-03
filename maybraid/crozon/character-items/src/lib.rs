//! Items that map onto characters.
//!
//! This crate owns the catalog of wearable items (currently clothing), the
//! shared item color palette, and per-item color selection. Species crates
//! describe *how* an item attaches to a particular character (rig, slot,
//! normalization); this crate describes *what* items exist and how they are
//! presented to menus.

pub mod clothing;
pub mod palette;

mod menu_traits;

pub use clothing::{ClothingColor, ClothingMesh};
pub use palette::ItemColor;
