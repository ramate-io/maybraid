//! Items that map onto characters.
//!
//! This crate owns the catalog of wearable and carried items (clothing and
//! firearms), the shared item color palette, clothing surface recipes, hashed
//! display names, and the inventory bag used at character creation. Species
//! crates describe *how* an item attaches to a particular character (rig, slot,
//! normalization); this crate describes *what* items exist and how they are
//! presented to menus.

pub mod clothing;
pub mod clothing_material;
pub mod firearm;
pub mod firearm_material;
pub mod firearm_roll;
pub mod inventory;
pub mod names;
pub mod palette;
pub mod stats;

mod menu_traits;

pub use clothing::{ClothingColor, ClothingHost, ClothingKind, ClothingMesh, ClothingSlot};
pub use clothing_material::{ClothingMaterial, ClothingMaterialChoice};
pub use firearm::{
	FirearmBarrel, FirearmGrip, FirearmKitSpec, FirearmMesh, FirearmScales, FirearmSpec,
	FirearmStock, FirearmTriggerBox, SlotScale, LENGTH_MILLI_MAX, LENGTH_MILLI_MIN,
	SCALE_MILLI_UNIT, THICKNESS_MILLI_MAX, THICKNESS_MILLI_MIN,
};
pub use firearm_material::{BoltMaterial, FirearmMaterial};
pub use firearm_roll::{generate_firearm_stats, Dist, FirearmBuff, FirearmPriors};
pub use inventory::{
	random_clothing_item, random_starter_clothing, random_starter_firearms, random_starter_loadout,
	Inventory, InventoryItem, InventorySlot, ItemRng, MaterialRefParams, STARTER_CLOTHING_COUNT,
	STARTER_WEAPON_COUNT, WEAPON_QUEUE_LIMIT, WORN_CLOTHING_LIMIT,
};
pub use names::{hashed_firearm_name, hashed_item_name};
pub use palette::ItemColor;
pub use stats::{CharacterSheet, ClothingStats, FireMode, FirearmStats, ProjectileKind};
