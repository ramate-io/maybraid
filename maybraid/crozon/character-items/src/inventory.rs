//! Owned items a character can wear or carry.
//!
//! The bag is a flat [`InventoryItem`] list. Each item belongs to one
//! [`InventorySlot`]; that slot holds an ordered unique selection of bag
//! indices (clothing wear order, weapons switch queue).

use bevy::prelude::*;

use crate::{
	hashed_firearm_name, hashed_item_name, ClothingKind, ClothingMaterial, ClothingMesh,
	ClothingStats, FirearmMesh, FirearmSpec, FirearmStats, ItemColor,
};

/// How many garments character creation rolls before the body editor.
pub const STARTER_CLOTHING_COUNT: usize = 3;

/// How many firearms character creation rolls into the bag.
pub const STARTER_WEAPON_COUNT: usize = 2;

/// Hard cap on simultaneously worn clothing items.
pub const WORN_CLOTHING_LIMIT: usize = 6;

/// Hard cap on the active weapon queue. Index 0 is the primary.
pub const WEAPON_QUEUE_LIMIT: usize = 3;

/// Bag partition. Slots are typed; items are not stored in separate vecs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InventorySlot {
	Clothing,
	Weapons,
}

impl InventorySlot {
	pub const fn capacity(self) -> usize {
		match self {
			Self::Clothing => WORN_CLOTHING_LIMIT,
			Self::Weapons => WEAPON_QUEUE_LIMIT,
		}
	}

	pub const fn label(self) -> &'static str {
		match self {
			Self::Clothing => "Clothing",
			Self::Weapons => "Weapons",
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

/// One owned instance in a character inventory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InventoryItem {
	Clothing { mesh: ClothingMesh, material: MaterialRefParams, stats: ClothingStats },
	Firearm { spec: FirearmSpec, stats: FirearmStats },
}

impl InventoryItem {
	pub fn clothing(mesh: ClothingMesh, material: ClothingMaterial, color: ItemColor) -> Self {
		Self::Clothing {
			mesh,
			material: MaterialRefParams::new(material, color),
			stats: ClothingStats::generate(mesh, material, color),
		}
	}

	pub fn firearm(mesh: FirearmMesh) -> Self {
		Self::from_firearm_spec(FirearmSpec::from_mesh(mesh))
	}

	pub fn from_firearm_spec(spec: FirearmSpec) -> Self {
		Self::Firearm { spec, stats: FirearmStats::generate(&spec) }
	}

	pub const fn slot(&self) -> InventorySlot {
		match self {
			Self::Clothing { .. } => InventorySlot::Clothing,
			Self::Firearm { .. } => InventorySlot::Weapons,
		}
	}

	pub const fn mesh(&self) -> Option<ClothingMesh> {
		match self {
			Self::Clothing { mesh, .. } => Some(*mesh),
			Self::Firearm { .. } => None,
		}
	}

	pub const fn firearm_mesh(&self) -> Option<FirearmMesh> {
		match self {
			Self::Firearm { spec, .. } => Some(spec.kit.body),
			Self::Clothing { .. } => None,
		}
	}

	pub const fn firearm_spec(&self) -> Option<FirearmSpec> {
		match self {
			Self::Firearm { spec, .. } => Some(*spec),
			Self::Clothing { .. } => None,
		}
	}

	pub const fn material(&self) -> Option<MaterialRefParams> {
		match self {
			Self::Clothing { material, .. } => Some(*material),
			Self::Firearm { .. } => None,
		}
	}

	pub const fn clothing_stats(&self) -> Option<ClothingStats> {
		match self {
			Self::Clothing { stats, .. } => Some(*stats),
			Self::Firearm { .. } => None,
		}
	}

	pub const fn firearm_stats(&self) -> Option<FirearmStats> {
		match self {
			Self::Firearm { stats, .. } => Some(*stats),
			Self::Clothing { .. } => None,
		}
	}

	pub fn catalog_detail(&self) -> String {
		match self {
			Self::Clothing { stats, .. } => stats.catalog_detail(),
			Self::Firearm { stats, .. } => stats.catalog_detail(),
		}
	}

	pub fn stat_rows(&self) -> Vec<(String, String)> {
		match self {
			Self::Clothing { stats, .. } => stats.stat_rows(),
			Self::Firearm { stats, .. } => stats.stat_rows(),
		}
	}

	pub const fn label(&self) -> &'static str {
		match self {
			Self::Clothing { mesh, .. } => mesh.label(),
			Self::Firearm { spec, .. } => spec.kit.body.label(),
		}
	}

	pub fn name(&self) -> String {
		match self {
			Self::Clothing { mesh, material, .. } => {
				hashed_item_name(*mesh, material.id, material.color)
			}
			Self::Firearm { spec, .. } => hashed_firearm_name(*spec),
		}
	}

	pub const fn path(&self) -> &'static str {
		match self {
			Self::Clothing { mesh, .. } => mesh.path(),
			Self::Firearm { spec, .. } => spec.kit.body.path(),
		}
	}
}

/// Owned items plus per-slot selections. Clothing is wear order; weapons are
/// the switch queue (first selected is primary).
#[derive(Component, Clone, Debug, Default, PartialEq, Eq)]
pub struct Inventory {
	pub items: Vec<InventoryItem>,
	pub clothing: Vec<usize>,
	pub weapons: Vec<usize>,
}

impl Inventory {
	/// Every clothing item starts worn (up to the wear cap). Weapons stay unequipped.
	pub fn with_all_worn(items: Vec<InventoryItem>) -> Self {
		let clothing: Vec<usize> = items
			.iter()
			.enumerate()
			.filter(|(_, item)| item.slot() == InventorySlot::Clothing)
			.map(|(index, _)| index)
			.take(InventorySlot::Clothing.capacity())
			.collect();
		Self { items, clothing, weapons: Vec::new() }
	}

	/// Create-mode bag: wear the first lower and first upper; queue every
	/// rolled firearm (up to the weapon cap). Extra clothing stays in the bag.
	pub fn with_starter_outfit(items: Vec<InventoryItem>) -> Self {
		let mut clothing = Vec::new();
		if let Some(index) = items
			.iter()
			.position(|item| item.mesh().is_some_and(|mesh| mesh.kind() == ClothingKind::Lower))
		{
			clothing.push(index);
		}
		if let Some(index) = items
			.iter()
			.position(|item| item.mesh().is_some_and(|mesh| mesh.kind() == ClothingKind::Upper))
		{
			clothing.push(index);
		}
		let weapons: Vec<usize> = items
			.iter()
			.enumerate()
			.filter(|(_, item)| item.slot() == InventorySlot::Weapons)
			.map(|(index, _)| index)
			.take(InventorySlot::Weapons.capacity())
			.collect();
		Self { items, clothing, weapons }
	}

	pub fn selected(&self, slot: InventorySlot) -> &[usize] {
		match slot {
			InventorySlot::Clothing => &self.clothing,
			InventorySlot::Weapons => &self.weapons,
		}
	}

	fn selected_mut(&mut self, slot: InventorySlot) -> &mut Vec<usize> {
		match slot {
			InventorySlot::Clothing => &mut self.clothing,
			InventorySlot::Weapons => &mut self.weapons,
		}
	}

	/// Clothing wear list (compat with the old `worn` field).
	pub fn worn(&self) -> &[usize] {
		&self.clothing
	}

	pub fn worn_items(&self) -> impl Iterator<Item = &InventoryItem> {
		self.clothing.iter().filter_map(|&index| self.items.get(index))
	}

	pub fn is_worn(&self, index: usize) -> bool {
		self.clothing.contains(&index)
	}

	/// 1-based rank in the item's slot, if selected.
	pub fn rank(&self, index: usize) -> Option<u8> {
		let item = self.items.get(index)?;
		self.selected(item.slot())
			.iter()
			.position(|&selected| selected == index)
			.map(|position| (position + 1) as u8)
	}

	pub fn primary_weapon(&self) -> Option<&InventoryItem> {
		self.weapons.first().and_then(|&index| self.items.get(index))
	}

	/// Wear / queue or remove `index` in its slot. At capacity, selecting a
	/// new item is a no-op. Returns whether the slot changed.
	pub fn toggle(&mut self, index: usize) -> bool {
		let Some(item) = self.items.get(index) else {
			return false;
		};
		let slot = item.slot();
		let selected = self.selected_mut(slot);
		if let Some(position) = selected.iter().position(|&selected| selected == index) {
			selected.remove(position);
			return true;
		}
		if selected.len() >= slot.capacity() {
			return false;
		}
		selected.push(index);
		true
	}

	/// Clothing-only name for [`Self::toggle`].
	pub fn toggle_worn(&mut self, index: usize) -> bool {
		self.toggle(index)
	}

	pub fn character_sheet(&self) -> crate::CharacterSheet {
		crate::CharacterSheet::from_inventory(self)
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

	/// Inclusive range. `max < min` returns `min`.
	pub fn in_range(&mut self, min: u32, max: u32) -> u32 {
		if max <= min {
			return min;
		}
		min + (self.next_u64() % (u64::from(max) - u64::from(min) + 1)) as u32
	}

	pub fn in_range_i16(&mut self, min: i16, max: i16) -> i16 {
		if max <= min {
			return min;
		}
		let span = (i32::from(max) - i32::from(min) + 1) as u32;
		min.saturating_add(self.in_range(0, span - 1) as i16)
	}

	/// Uniform in `[0, 1)`.
	pub fn unit(&mut self) -> f32 {
		(self.next_u64() >> 11) as f32 / ((1u64 << 53) as f32)
	}

	/// Box–Muller sample of \(\mathcal{N}(\mu, \sigma)\).
	pub fn sample_normal(&mut self, mean: f32, sd: f32) -> f32 {
		let u1 = self.unit().max(1e-7);
		let u2 = self.unit();
		let z = (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos();
		mean + sd * z
	}

	pub fn choose<'a, T>(&mut self, values: &'a [T]) -> Option<&'a T> {
		if values.is_empty() {
			return None;
		}
		Some(&values[self.gen_index(values.len())])
	}
}

/// One random clothing item (mesh, look, color). Stats roll from identity.
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

/// Unique firearms for the starter bag.
pub fn random_starter_firearms(rng: &mut ItemRng, count: usize) -> Vec<InventoryItem> {
	let mut remaining: Vec<FirearmMesh> = FirearmMesh::VALUES.to_vec();
	let mut items = Vec::new();
	let take = count.min(remaining.len());
	for _ in 0..take {
		let index = rng.gen_index(remaining.len());
		let mesh = remaining.swap_remove(index);
		items.push(InventoryItem::from_firearm_spec(FirearmSpec::roll(rng, mesh)));
	}
	items
}

/// Clothing starter plus two unique firearms.
pub fn random_starter_loadout(rng: &mut ItemRng) -> Vec<InventoryItem> {
	let mut items = random_starter_clothing(rng, STARTER_CLOTHING_COUNT);
	items.extend(random_starter_firearms(rng, STARTER_WEAPON_COUNT));
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
		assert!(items
			.iter()
			.all(|item| item.clothing_stats().is_some_and(|stats| stats.weight > 0)));
		assert_eq!(items[0].mesh().map(ClothingMesh::kind), Some(ClothingKind::Lower));
		assert_eq!(items[1].mesh().map(ClothingMesh::kind), Some(ClothingKind::Upper));
		assert!(ClothingKind::STARTER_LOWERS.contains(&items[0].mesh().unwrap()));
		assert!(ClothingKind::STARTER_UPPERS.contains(&items[1].mesh().unwrap()));
	}

	#[test]
	fn starter_outfit_wears_lower_and_upper_and_queues_guns() {
		let items = vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::clothing(ClothingMesh::TankTop, ClothingMaterial::Cloth, ItemColor::Red),
			InventoryItem::clothing(ClothingMesh::Robe, ClothingMaterial::Cloth, ItemColor::Cool),
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::firearm(FirearmMesh::Reltor),
		];
		let inventory = Inventory::with_starter_outfit(items);
		assert_eq!(inventory.clothing, vec![0, 1]);
		assert_eq!(inventory.weapons, vec![3, 4]);
		assert!(!inventory.is_worn(2));
		assert_eq!(inventory.rank(3), Some(1));
		assert_eq!(inventory.rank(4), Some(2));
		assert_eq!(
			inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
	}

	#[test]
	fn wear_cap_rejects_a_seventh_item() {
		let items: Vec<_> = ClothingMesh::VALUES
			.iter()
			.take(WORN_CLOTHING_LIMIT + 1)
			.map(|mesh| InventoryItem::clothing(*mesh, ClothingMaterial::Cloth, ItemColor::Natural))
			.collect();
		let mut inventory = Inventory::with_all_worn(items);
		assert_eq!(inventory.clothing.len(), WORN_CLOTHING_LIMIT);
		assert!(!inventory.toggle_worn(WORN_CLOTHING_LIMIT));
		assert!(inventory.toggle_worn(0));
		assert_eq!(inventory.clothing.len(), WORN_CLOTHING_LIMIT - 1);
		assert!(inventory.toggle_worn(WORN_CLOTHING_LIMIT));
	}

	#[test]
	fn weapon_queue_caps_at_three_and_compacts_rank() {
		let items: Vec<_> =
			FirearmMesh::VALUES.iter().map(|mesh| InventoryItem::firearm(*mesh)).collect();
		let mut inventory = Inventory { items, clothing: Vec::new(), weapons: vec![0, 1, 2] };
		assert!(!inventory.toggle(3));
		assert!(inventory.toggle(1));
		assert_eq!(inventory.weapons, vec![0, 2]);
		assert_eq!(inventory.rank(0), Some(1));
		assert_eq!(inventory.rank(2), Some(2));
		assert!(inventory.toggle(3));
		assert_eq!(inventory.weapons, vec![0, 2, 3]);
	}

	#[test]
	fn starter_loadout_has_clothes_and_two_guns() {
		let items = random_starter_loadout(&mut ItemRng::from_seed(42));
		assert_eq!(items.len(), STARTER_CLOTHING_COUNT + STARTER_WEAPON_COUNT);
		assert_eq!(items.iter().filter(|item| item.slot() == InventorySlot::Clothing).count(), 3);
		assert_eq!(items.iter().filter(|item| item.slot() == InventorySlot::Weapons).count(), 2);
		let mut guns: Vec<_> = items.iter().filter_map(InventoryItem::firearm_mesh).collect();
		guns.sort_by_key(|mesh| mesh.label());
		guns.dedup();
		assert_eq!(guns.len(), 2);
	}

	#[test]
	fn seeded_rng_is_deterministic() {
		let a = random_starter_loadout(&mut ItemRng::from_seed(42));
		let b = random_starter_loadout(&mut ItemRng::from_seed(42));
		assert_eq!(a, b);
		assert_ne!(a[0].name(), a[0].label());
	}
}
