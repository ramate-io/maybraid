//! Inventory bag as a 1:1 Bevy relationship, persisted beside the character.

use bevy::prelude::*;
use crozon_character_items::{
	ClothingMaterial, ClothingMesh, Inventory, InventoryItem, ItemColor, WORN_CLOTHING_LIMIT,
};
use crozon_character_persist::{CharacterId, PersistError, SaveRoot};
use serde::{Deserialize, Serialize};
use std::fs;

const VERSION: u32 = 1;

/// Capsule/session using an inventory bag.
///
/// 1:1 onto the bag entity (`bag`). Inserting it stamps [`CarriedBy`] so
/// despawn/replace stays consistent.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = CarriedBy)]
pub struct InventoryUser {
	#[relationship]
	pub bag: Entity,
	pub settings: InventoryUserSettings,
}

impl InventoryUser {
	pub fn carrying(bag: Entity) -> Self {
		Self { bag, settings: InventoryUserSettings::default() }
	}
}

#[derive(Clone, Copy, Debug)]
pub struct InventoryUserSettings {
	pub worn_limit: usize,
}

impl Default for InventoryUserSettings {
	fn default() -> Self {
		Self { worn_limit: WORN_CLOTHING_LIMIT }
	}
}

/// Bag-side 1:1 target of [`InventoryUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = InventoryUser)]
pub struct CarriedBy(Entity);

pub struct InventoryUserPlugin;

impl Plugin for InventoryUserPlugin {
	fn build(&self, _app: &mut App) {}
}

#[derive(Serialize, Deserialize)]
struct InventoryFile {
	version: u32,
	id: CharacterId,
	items: Vec<ClothingItemFile>,
	worn: Vec<usize>,
}

#[derive(Serialize, Deserialize)]
struct ClothingItemFile {
	mesh: ClothingMesh,
	material: ClothingMaterial,
	color: ItemColor,
}

impl ClothingItemFile {
	fn from_item(item: &InventoryItem) -> Option<Self> {
		Some(Self {
			mesh: item.mesh()?,
			material: item.material().id,
			color: item.material().color,
		})
	}

	fn into_item(self) -> InventoryItem {
		InventoryItem::clothing(self.mesh, self.material, self.color)
	}
}

/// Write `inventories/{id}.json`. Missing parent dirs are created.
pub fn save(root: &SaveRoot, id: CharacterId, inventory: &Inventory) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let file = InventoryFile {
		version: VERSION,
		id,
		items: inventory.items.iter().filter_map(ClothingItemFile::from_item).collect(),
		worn: inventory.worn.clone(),
	};
	let json = serde_json::to_string_pretty(&file)?;
	fs::write(root.inventory_path(id), json)?;
	Ok(())
}

/// Load the bag. A missing file is an empty inventory, not an error.
pub fn load(root: &SaveRoot, id: CharacterId) -> Result<Inventory, PersistError> {
	let path = root.inventory_path(id);
	let json = match fs::read_to_string(&path) {
		Ok(json) => json,
		Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
			return Ok(Inventory::default());
		}
		Err(error) => return Err(error.into()),
	};
	let file: InventoryFile = serde_json::from_str(&json)?;
	let items: Vec<InventoryItem> =
		file.items.into_iter().map(ClothingItemFile::into_item).collect();
	let worn: Vec<usize> = file
		.worn
		.into_iter()
		.filter(|&index| index < items.len())
		.take(WORN_CLOTHING_LIMIT)
		.collect();
	Ok(Inventory { items, worn })
}

pub fn spawn_bag(commands: &mut Commands, host: Entity, inventory: Inventory) -> Entity {
	let bag = commands.spawn(inventory).id();
	commands.entity(host).insert(InventoryUser::carrying(bag));
	bag
}

#[cfg(test)]
mod tests {
	use super::*;
	use crozon_character_items::{random_starter_clothing, ClothingMesh, ItemRng};

	#[test]
	fn starter_outfit_round_trips() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		let items = random_starter_clothing(&mut ItemRng::from_seed(3), 3);
		let inventory = Inventory::with_starter_outfit(items);
		save(&root, id, &inventory).expect("save");
		let loaded = load(&root, id).expect("load");
		assert_eq!(loaded, inventory);
		assert!(loaded.items.iter().all(|item| item.buffs().is_empty()));
	}

	#[test]
	fn missing_file_is_empty_bag() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let loaded = load(&root, CharacterId(1)).expect("load");
		assert_eq!(loaded, Inventory::default());
	}

	#[test]
	fn clothing_labels_are_kebab_case() {
		assert_eq!(ClothingMesh::TankTop.label(), "tank-top");
		let json = serde_json::to_string(&ClothingMesh::TankTop).expect("json");
		assert_eq!(json, "\"tank-top\"");
	}
}
