//! Owned items a character can wear or carry.
//!
//! Worn clothing is still assembled as [`crate::clothing`] layers. This module
//! is the bag those layers are chosen from. Stats/buffs are stubbed until the
//! in-game inventory panel lands.

use crate::{ClothingMaterial, ClothingMesh, ItemColor};

/// How many garments character creation rolls before the body editor.
pub const STARTER_CLOTHING_COUNT: usize = 3;

/// Hard cap on simultaneously worn clothing items (create-a-character and later
/// the in-game panel). Starter rolls fit under this.
pub const WORN_CLOTHING_LIMIT: usize = 6;

/// Catalog identity of an owned item. Clothing is the only kind today.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Item {
	Clothing(ClothingMesh),
}

impl Item {
	pub const fn clothing(self) -> Option<ClothingMesh> {
		match self {
			Self::Clothing(mesh) => Some(mesh),
		}
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Clothing(mesh) => mesh.label(),
		}
	}

	pub const fn path(self) -> &'static str {
		match self {
			Self::Clothing(mesh) => mesh.path(),
		}
	}
}

/// Recipe name plus palette[0] used to rebuild a clothing [`material_ref::MaterialRef`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialRefParams {
	pub id: ClothingMaterial,
	pub color: ItemColor,
}

impl MaterialRefParams {
	pub const fn new(id: ClothingMaterial, color: ItemColor) -> Self {
		Self { id, color }
	}
}

/// Gameplay modifier on an item. Empty until a stats pass lands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Buff {}

/// One slot in a character inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InventoryItem {
	Clothing { item: Item, material: MaterialRefParams, buffs: Vec<Buff> },
}

impl InventoryItem {
	pub fn clothing(mesh: ClothingMesh, material: ClothingMaterial, color: ItemColor) -> Self {
		Self::Clothing {
			item: Item::Clothing(mesh),
			material: MaterialRefParams::new(material, color),
			buffs: Vec::new(),
		}
	}

	pub const fn item(&self) -> Item {
		match self {
			Self::Clothing { item, .. } => *item,
		}
	}

	pub const fn mesh(&self) -> Option<ClothingMesh> {
		self.item().clothing()
	}

	pub const fn material(&self) -> MaterialRefParams {
		match self {
			Self::Clothing { material, .. } => *material,
		}
	}

	pub fn buffs(&self) -> &[Buff] {
		match self {
			Self::Clothing { buffs, .. } => buffs,
		}
	}

	pub const fn label(&self) -> &'static str {
		self.item().label()
	}

	pub const fn path(&self) -> &'static str {
		self.item().path()
	}
}

/// Owned items plus which of them are worn. Worn indices are unique and stay
/// within [`WORN_CLOTHING_LIMIT`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
	pub items: Vec<InventoryItem>,
	pub worn: Vec<usize>,
}

impl Inventory {
	/// Every item starts worn (up to the wear cap). Used for starter rolls.
	pub fn with_all_worn(items: Vec<InventoryItem>) -> Self {
		let worn: Vec<usize> = (0..items.len().min(WORN_CLOTHING_LIMIT)).collect();
		Self { items, worn }
	}

	pub fn worn_items(&self) -> impl Iterator<Item = &InventoryItem> {
		self.worn.iter().filter_map(|&index| self.items.get(index))
	}

	pub fn is_worn(&self, index: usize) -> bool {
		self.worn.contains(&index)
	}

	/// Wear or unwear `index`. Wearing when already at the cap is a no-op.
	/// Returns whether the worn set changed.
	pub fn toggle_worn(&mut self, index: usize) -> bool {
		if index >= self.items.len() {
			return false;
		}
		if let Some(slot) = self.worn.iter().position(|&worn| worn == index) {
			self.worn.remove(slot);
			return true;
		}
		if self.worn.len() >= WORN_CLOTHING_LIMIT {
			return false;
		}
		self.worn.push(index);
		true
	}
}

/// Tiny xorshift64* so this crate does not take on `rand`.
#[derive(Clone, Debug)]
pub struct ItemRng(u64);

impl ItemRng {
	pub const fn from_seed(seed: u64) -> Self {
		Self(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
	}

	pub fn from_entropy() -> Self {
		let nanos = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.map(|elapsed| elapsed.as_nanos() as u64)
			.unwrap_or(0x9E37_79B9_7F4A_7C15);
		Self::from_seed(nanos.wrapping_mul(0xA24B_AED4_96E9_C13F) ^ 0xD1B5_4A32_D192_ED03)
	}

	fn next_u64(&mut self) -> u64 {
		let mut x = self.0;
		x ^= x >> 12;
		x ^= x << 25;
		x ^= x >> 27;
		self.0 = x;
		x.wrapping_mul(0x2545_F491_4F6C_DD1D)
	}

	pub fn gen_index(&mut self, len: usize) -> usize {
		if len <= 1 {
			return 0;
		}
		(self.next_u64() as usize) % len
	}

	pub fn choose<'a, T>(&mut self, values: &'a [T]) -> Option<&'a T> {
		if values.is_empty() {
			return None;
		}
		Some(&values[self.gen_index(values.len())])
	}
}

/// One random clothing item (mesh, look, color). Buffs stay empty.
pub fn random_clothing_item(rng: &mut ItemRng) -> InventoryItem {
	let mesh = *rng.choose(ClothingMesh::VALUES).unwrap_or(&ClothingMesh::TankTop);
	let material = *rng.choose(ClothingMaterial::VALUES).unwrap_or(&ClothingMaterial::Cloth);
	let color = *rng.choose(ItemColor::VALUES).unwrap_or(&ItemColor::Natural);
	InventoryItem::clothing(mesh, material, color)
}

/// `count` clothing items with unique meshes when the catalog is large enough.
pub fn random_starter_clothing(rng: &mut ItemRng, count: usize) -> Vec<InventoryItem> {
	let mut remaining: Vec<ClothingMesh> = ClothingMesh::VALUES.to_vec();
	let take = count.min(remaining.len());
	let mut items = Vec::with_capacity(take);
	for _ in 0..take {
		let index = rng.gen_index(remaining.len());
		let mesh = remaining.swap_remove(index);
		let material = *rng.choose(ClothingMaterial::VALUES).unwrap_or(&ClothingMaterial::Cloth);
		let color = *rng.choose(ItemColor::VALUES).unwrap_or(&ItemColor::Natural);
		items.push(InventoryItem::clothing(mesh, material, color));
	}
	items
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn starter_rolls_are_unique_meshes() {
		let items = random_starter_clothing(&mut ItemRng::from_seed(7), STARTER_CLOTHING_COUNT);
		assert_eq!(items.len(), STARTER_CLOTHING_COUNT);
		let mut meshes: Vec<_> = items.iter().filter_map(InventoryItem::mesh).collect();
		meshes.sort_by_key(|mesh| mesh.label());
		meshes.dedup();
		assert_eq!(meshes.len(), STARTER_CLOTHING_COUNT);
		assert!(items.iter().all(|item| item.buffs().is_empty()));
	}

	#[test]
	fn wear_cap_rejects_a_seventh_item() {
		let items: Vec<_> = ClothingMesh::VALUES
			.iter()
			.take(WORN_CLOTHING_LIMIT + 1)
			.map(|mesh| InventoryItem::clothing(*mesh, ClothingMaterial::Cloth, ItemColor::Natural))
			.collect();
		let mut inventory = Inventory::with_all_worn(items);
		assert_eq!(inventory.worn.len(), WORN_CLOTHING_LIMIT);
		assert!(!inventory.toggle_worn(WORN_CLOTHING_LIMIT));
		assert!(inventory.toggle_worn(0));
		assert_eq!(inventory.worn.len(), WORN_CLOTHING_LIMIT - 1);
		assert!(inventory.toggle_worn(WORN_CLOTHING_LIMIT));
	}

	#[test]
	fn seeded_rng_is_deterministic() {
		let a = random_starter_clothing(&mut ItemRng::from_seed(42), 3);
		let b = random_starter_clothing(&mut ItemRng::from_seed(42), 3);
		assert_eq!(a, b);
	}
}
