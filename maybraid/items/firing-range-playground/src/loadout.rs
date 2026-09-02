//! Roll a generated clothing + firearm identity for a firing-range combatant.

use crozon_character_items::{
	random_starter_loadout, FirearmBarrel, FirearmGrip, FirearmKitSpec, FirearmMesh, FirearmSpec,
	FirearmStock, FirearmTriggerBox, Inventory, InventoryItem, ItemRng,
};
use crozon_characters::species::braidman::BraidmanConfig;
use firearms::{BarrelMesh, BodyMesh, FirearmKit, GripMesh, StockMesh, TriggerBoxMesh};

/// Clothing recipe plus the combat kit assembled from a rolled firearm spec.
#[derive(Clone, Debug)]
pub(crate) struct CombatantLoadout {
	pub appearance: BraidmanConfig,
	pub kit: FirearmKit,
}

pub(crate) fn roll_combatant(rng: &mut ItemRng) -> CombatantLoadout {
	let inventory = Inventory::with_starter_outfit(random_starter_loadout(rng));
	CombatantLoadout {
		appearance: appearance_from_inventory(&inventory),
		kit: inventory
			.primary_weapon()
			.and_then(InventoryItem::firearm_spec)
			.map(kit_from_spec)
			.unwrap_or_else(|| FirearmKit::body(BodyMesh::Bullpup)),
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

	#[test]
	fn starter_loadout_wears_clothes_and_queues_a_gun() {
		let loadout = roll_combatant(&mut ItemRng::from_seed(11));
		assert!(!loadout.appearance.clothing.is_empty());
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
}
