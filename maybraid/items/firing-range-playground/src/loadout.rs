//! Roll a generated clothing + firearm identity for a firing-range combatant.

use crozon_character_items::{
	random_starter_clothing, realize_firearm_stats, CharacterSheet, FirearmBarrel, FirearmGrip,
	FirearmKitSpec, FirearmMesh, FirearmSpec, FirearmStats, FirearmStock, FirearmTriggerBox,
	Inventory, InventoryItem, ItemRng, STARTER_CLOTHING_COUNT,
};
use crozon_characters::species::braidman::BraidmanConfig;
use firearms::{BarrelMesh, BodyMesh, FirearmKit, GripMesh, StockMesh, TriggerBoxMesh};

/// Clothing recipe plus the combat kit assembled from a rolled firearm spec.
#[derive(Clone, Debug)]
pub(crate) struct CombatantLoadout {
	pub appearance: BraidmanConfig,
	pub spec: FirearmSpec,
	pub kit: FirearmKit,
	pub stats: FirearmStats,
	pub sheet: CharacterSheet,
}

pub(crate) fn roll_combatant(rng: &mut ItemRng) -> CombatantLoadout {
	let mut items = random_starter_clothing(rng, STARTER_CLOTHING_COUNT);
	let body = *rng.choose(FirearmMesh::VALUES).unwrap_or(&FirearmMesh::Bullpup);
	let spec = FirearmSpec::roll(rng, body);
	let stats = realize_firearm_stats(rng, &spec);
	items.push(InventoryItem::Firearm { spec, stats });
	let inventory = Inventory::with_starter_outfit(items);
	CombatantLoadout {
		appearance: appearance_from_inventory(&inventory),
		spec,
		kit: kit_from_spec(spec),
		stats,
		sheet: CharacterSheet::from_inventory(&inventory),
	}
}

fn appearance_from_inventory(inventory: &Inventory) -> BraidmanConfig {
	let mut config = BraidmanConfig::default_preview();
	for item in inventory.worn_items() {
		let InventoryItem::Clothing { mesh, material, .. } = item else {
			continue;
		};
		if !config.clothing.contains(mesh) {
			config.clothing.push(*mesh);
		}
		config.colors.set_clothing_color(*mesh, material.color);
		config.colors.set_clothing_material(*mesh, material.id);
	}
	config
}

pub(crate) fn kit_from_spec(spec: FirearmSpec) -> FirearmKit {
	kit_from_parts(spec.kit)
}

fn kit_from_parts(kit: FirearmKitSpec) -> FirearmKit {
	FirearmKit {
		body: body_mesh(kit.body),
		barrel: barrel_mesh(kit.barrel),
		trigger_box: trigger_box_mesh(kit.trigger_box),
		grip: grip_mesh(kit.grip),
		stock: stock_mesh(kit.stock),
	}
}

fn body_mesh(mesh: FirearmMesh) -> BodyMesh {
	match mesh {
		FirearmMesh::Bullpup => BodyMesh::Bullpup,
		FirearmMesh::Silopup => BodyMesh::Silopup,
		FirearmMesh::Reltor => BodyMesh::Reltor,
		FirearmMesh::Samsonist => BodyMesh::Samsonist,
		FirearmMesh::Snailer => BodyMesh::Snailer,
	}
}

fn barrel_mesh(mesh: FirearmBarrel) -> BarrelMesh {
	match mesh {
		FirearmBarrel::None => BarrelMesh::None,
		FirearmBarrel::Bullpup => BarrelMesh::Bullpup,
		FirearmBarrel::Laznard => BarrelMesh::Laznard,
	}
}

fn grip_mesh(mesh: FirearmGrip) -> GripMesh {
	match mesh {
		FirearmGrip::None => GripMesh::None,
		FirearmGrip::BumpHandle => GripMesh::BumpHandle,
	}
}

fn trigger_box_mesh(mesh: FirearmTriggerBox) -> TriggerBoxMesh {
	match mesh {
		FirearmTriggerBox::None => TriggerBoxMesh::None,
		FirearmTriggerBox::Keelripe => TriggerBoxMesh::Keelripe,
		FirearmTriggerBox::Paddle => TriggerBoxMesh::Paddle,
		FirearmTriggerBox::Reltor => TriggerBoxMesh::Reltor,
	}
}

fn stock_mesh(mesh: FirearmStock) -> StockMesh {
	match mesh {
		FirearmStock::None => StockMesh::None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crozon_character_items::{ItemColor, ProjectileKind};
	use std::collections::BTreeSet;

	#[test]
	fn starter_loadout_wears_clothes_and_queues_a_gun() {
		let loadout = roll_combatant(&mut ItemRng::from_seed(11));
		assert!(!loadout.appearance.clothing.is_empty());
		assert!(loadout.stats.damage > 0);
		assert!(loadout.sheet.health >= 1);
	}

	#[test]
	fn kit_mapping_preserves_bullpup_concept_slots() {
		let kit = kit_from_spec(FirearmSpec::from_mesh(FirearmMesh::Bullpup));
		assert_eq!(kit.body, BodyMesh::Bullpup);
		assert_eq!(kit.barrel, BarrelMesh::Bullpup);
		assert_eq!(kit.grip, GripMesh::BumpHandle);
	}

	#[test]
	fn kit_mapping_preserves_reltor_trigger_box() {
		let kit = kit_from_spec(FirearmSpec::from_mesh(FirearmMesh::Reltor));
		assert_eq!(kit.body, BodyMesh::Reltor);
		assert_eq!(kit.trigger_box, TriggerBoxMesh::Reltor);
	}

	#[test]
	fn gallery_style_rolls_are_visually_and_ballistically_diverse() {
		let mut rng = ItemRng::from_seed(7);
		let mut colors = BTreeSet::new();
		let mut kinds = BTreeSet::new();
		let mut greens = 0usize;
		let mut lasers = 0usize;
		const N: usize = 24;
		for _ in 0..N {
			let loadout = roll_combatant(&mut rng);
			colors.insert(loadout.spec.looks.body.color.label());
			kinds.insert(loadout.stats.projectile.label());
			if loadout.spec.looks.body.color == ItemColor::Green {
				greens += 1;
			}
			if loadout.stats.projectile == ProjectileKind::Laser {
				lasers += 1;
			}
		}
		assert!(colors.len() >= 4, "body colors {colors:?}");
		assert!(kinds.len() >= 2, "projectiles {kinds:?}");
		assert!(greens < N / 2, "green {greens}/{N}");
		assert!(lasers < (N * 3) / 4, "lasers {lasers}/{N}");
	}
}
