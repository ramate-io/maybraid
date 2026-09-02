//! Items that map onto characters.
//!
//! This crate owns the catalog of wearable items (currently clothing), the
//! shared item color palette, clothing surface recipes, per-item color
//! selection, hashed display names, and the inventory bag used at character
//! creation. Species crates describe *how* an item attaches to a particular
//! character (rig, slot, normalization); this crate describes *what* items
//! exist and how they are presented to menus.

pub mod clothing;
pub mod clothing_material;
pub mod inventory;
pub mod names;
pub mod palette;

mod menu_traits;

pub use clothing::{ClothingColor, ClothingHost, ClothingMesh, ClothingSlot};
pub use clothing_material::{ClothingMaterial, ClothingMaterialChoice};
pub use inventory::{
	Buff, Inventory, InventoryItem, Item, ItemRng, MaterialRefParams, STARTER_CLOTHING_COUNT,
	WORN_CLOTHING_LIMIT, random_clothing_item, random_starter_clothing,
};
pub use names::hashed_item_name;
pub use palette::ItemColor;
