//! Owned items a character can wear or carry.
//!
//! Worn clothing is still assembled as [`crate::clothing`] layers. This module
//! is the bag those layers are chosen from. Stats/buffs are stubbed until the
//! in-game inventory panel lands.

use crate::{hashed_item_name, ClothingKind, ClothingMaterial, ClothingMesh, ItemColor};

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

	/// Hash-picked display name from look, color, and mesh word lists.
	pub fn name(&self) -> String {
		match self {
			Self::Clothing { item: Item::Clothing(mesh), material, .. } => {
				hashed_item_name(*mesh, material.id, material.color)
			}
		}
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
	/// Every item starts worn (up to the wear cap).
	pub fn with_all_worn(items: Vec<InventoryItem>) -> Self {
		let worn: Vec<usize> = (0..items.len().min(WORN_CLOTHING_LIMIT)).collect();
		Self { items, worn }
	}

	/// Create-mode bag: own the whole roll, wear the first lower and first upper
	/// so the editor opens fully clothed. The extra “any” item stays in the bag.
	pub fn with_starter_outfit(items: Vec<InventoryItem>) -> Self {
		let mut worn = Vec::new();
		if let Some(index) = items
			.iter()
			.position(|item| item.mesh().is_some_and(|mesh| mesh.kind() == ClothingKind::Lower))
		{
			worn.push(index);
		}
		if let Some(index) = items
			.iter()
			.position(|item| item.mesh().is_some_and(|mesh| mesh.kind() == ClothingKind::Upper))
		{
			worn.push(index);
		}
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
	random_item_for_mesh(rng, mesh)
}

fn random_item_for_mesh(rng: &mut ItemRng, mesh: ClothingMesh) -> InventoryItem {
	let material = *rng.choose(ClothingMaterial::VALUES).unwrap_or(&ClothingMaterial::Cloth);
	let color = *rng.choose(ItemColor::VALUES).unwrap_or(&ItemColor::Natural);
	InventoryItem::clothing(mesh, material, color)
}

/// Starter roll: one lower, one upper, then unique “any” fills to `count`.
///
/// Wear is not a restriction — [`Inventory::with_starter_outfit`] equips the
/// lower and upper so the body editor opens clothed.
pub fn random_starter_clothing(rng: &mut ItemRng, count: usize) -> Vec<InventoryItem> {
	let lower = *rng.choose(ClothingKind::STARTER_LOWERS).unwrap_or(&ClothingMesh::Pants);
	let upper = *rng.choose(ClothingKind::STARTER_UPPERS).unwrap_or(&ClothingMesh::TankTop);
	let mut items = vec![random_item_for_mesh(rng, lower), random_item_for_mesh(rng, upper)];
	let mut remaining: Vec<ClothingMesh> = ClothingMesh::VALUES
		.iter()
		.copied()
		.filter(|mesh| *mesh != lower && *mesh != upper)
		.collect();
	let extra = count.saturating_sub(items.len()).min(remaining.len());
	for _ in 0..extra {
		let index = rng.gen_index(remaining.len());
		let mesh = remaining.swap_remove(index);
		items.push(random_item_for_mesh(rng, mesh));
	}
	items
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{ClothingKind, ClothingMaterial, ClothingMesh, ItemColor};

	#[test]
	fn starter_rolls_are_unique_meshes() {
		let items = random_starter_clothing(&mut ItemRng::from_seed(7), STARTER_CLOTHING_COUNT);
		assert_eq!(items.len(), STARTER_CLOTHING_COUNT);
		let mut meshes: Vec<_> = items.iter().filter_map(InventoryItem::mesh).collect();
		meshes.sort_by_key(|mesh| mesh.label());
		meshes.dedup();
		assert_eq!(meshes.len(), STARTER_CLOTHING_COUNT);
		assert!(items.iter().all(|item| item.buffs().is_empty()));
		assert_eq!(items[0].mesh().map(ClothingMesh::kind), Some(ClothingKind::Lower));
		assert_eq!(items[1].mesh().map(ClothingMesh::kind), Some(ClothingKind::Upper));
		assert!(ClothingKind::STARTER_LOWERS.contains(&items[0].mesh().unwrap()));
		assert!(ClothingKind::STARTER_UPPERS.contains(&items[1].mesh().unwrap()));
	}

	#[test]
	fn starter_outfit_wears_lower_and_upper() {
		let items = vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
			InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Cloth, ItemColor::Cool),
		];
		let inventory = Inventory::with_starter_outfit(items);
		assert_eq!(inventory.worn, vec![0, 1]);
		assert!(!inventory.is_worn(2));
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
		assert_ne!(a[0].name(), a[0].label());
	}
}
