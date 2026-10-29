//! Unequip / hold requests. Item crates realize construction and teardown.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::Inventory;

use crate::dispatch::EquippedItemHandler;

/// Bag mutation equipment already committed. Hosts (world loadout) persist it.
#[derive(Message, Clone, Debug)]
pub struct EquipmentInventoryChanged {
	actor: Entity,
	inventory: Inventory,
}

impl EquipmentInventoryChanged {
	pub const fn new(actor: Entity, inventory: Inventory) -> Self {
		Self { actor, inventory }
	}

	pub const fn actor(&self) -> Entity {
		self.actor
	}

	pub fn inventory(&self) -> &Inventory {
		&self.inventory
	}

	pub fn into_inventory(self) -> Inventory {
		self.inventory
	}
}

/// Request that item crates tear down the actor's held kit.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PendingEquipmentTeardown;

impl PendingEquipmentTeardown {
	pub fn request(entity: &mut EntityCommands) {
		entity.insert(Self);
	}
}

#[derive(Clone, Debug)]
enum EquipmentHold {
	Inventory(Inventory),
	DefaultFirearm,
}

/// Request that item crates hold the inventory's primary weapon or the
/// explicit legacy default firearm.
#[derive(Component, Clone, Debug)]
pub struct PendingEquipmentHold(EquipmentHold);

/// Per-actor opt-in to keep a held kit aligned with an inventory bag.
///
/// Hosts choose which actors enroll; NPC inventories and standalone item
/// users are not reconciled unless the host adds this marker.
/// Reconciliation compares handler kinds. Selecting another item of the same
/// kind still requires a host or holster hold request to replace the held kit.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct ReconcileEquipmentFromInventory;

impl PendingEquipmentHold {
	pub fn new(inventory: Inventory) -> Self {
		Self(EquipmentHold::Inventory(inventory))
	}

	pub fn inventory(&self) -> Option<&Inventory> {
		match &self.0 {
			EquipmentHold::Inventory(inventory) => Some(inventory),
			EquipmentHold::DefaultFirearm => None,
		}
	}

	pub fn request(entity: &mut EntityCommands, inventory: Inventory) {
		EquippedItemHandler::from_inventory(&inventory).install(entity);
		entity.insert(Self(EquipmentHold::Inventory(inventory)));
	}

	/// Request that firearm-user realize its legacy default Bullpup kit.
	///
	/// The world fallback intentionally uses `LiveWeapon::default()` rather
	/// than catalog firearm stats, so this is an explicit request.
	pub fn request_default_firearm(entity: &mut EntityCommands) {
		EquippedItemHandler::Firearm.install(entity);
		entity.insert((PendingEquipmentTeardown, Self(EquipmentHold::DefaultFirearm)));
	}
}

/// Reconcile only actors explicitly enrolled by their host.
pub fn reconcile_equipment_from_inventory(
	mut commands: Commands,
	actors: Query<
		(
			Entity,
			Option<&InventoryUser>,
			Option<&EquippedItemHandler>,
			Has<PendingEquipmentHold>,
			Has<PendingEquipmentTeardown>,
		),
		With<ReconcileEquipmentFromInventory>,
	>,
	bags: Query<&Inventory>,
) {
	for (actor, user, installed, pending_hold, pending_teardown) in &actors {
		if pending_hold || pending_teardown {
			continue;
		}
		let inventory = user.and_then(|user| bags.get(user.bag).ok());
		let desired = inventory.map(EquippedItemHandler::from_inventory).unwrap_or_default();
		if installed.copied().unwrap_or_default() == desired {
			continue;
		}
		let snapshot = inventory.cloned().unwrap_or_default();
		let mut entity = commands.entity(actor);
		PendingEquipmentTeardown::request(&mut entity);
		PendingEquipmentHold::request(&mut entity, snapshot);
	}
}

/// Drop request components after item crates have realized them.
pub fn clear_equipment_requests(
	mut commands: Commands,
	teardown: Query<Entity, With<PendingEquipmentTeardown>>,
	hold: Query<Entity, With<PendingEquipmentHold>>,
) {
	for entity in &teardown {
		commands.entity(entity).remove::<PendingEquipmentTeardown>();
	}
	for entity in &hold {
		commands.entity(entity).remove::<PendingEquipmentHold>();
	}
}

/// Remove a stale handler for teardown-only requests. A paired hold already
/// stamped the incoming handler and must retain it until the item crates hold.
pub fn clear_handler_for_teardown_only(
	mut commands: Commands,
	actors: Query<(Entity, Has<PendingEquipmentHold>), With<PendingEquipmentTeardown>>,
) {
	for (actor, has_hold) in &actors {
		if !has_hold {
			EquippedItemHandler::clear(&mut commands.entity(actor));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use character_items::{FirearmMesh, InventoryItem};

	fn inventory(item: InventoryItem) -> Inventory {
		Inventory {
			items: vec![item],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		}
	}

	#[test]
	fn reconciliation_is_opt_in_and_uses_the_bag_selection() -> anyhow::Result<()> {
		let mut world = World::new();
		let bag = world.spawn(inventory(InventoryItem::Fisticuffs)).id();
		let enrolled = world
			.spawn((
				InventoryUser::carrying(bag),
				EquippedItemHandler::Firearm,
				ReconcileEquipmentFromInventory,
			))
			.id();
		let other_bag = world.spawn(inventory(InventoryItem::Fisticuffs)).id();
		let unenrolled = world
			.spawn((InventoryUser::carrying(other_bag), EquippedItemHandler::Firearm))
			.id();
		world
			.run_system_once(reconcile_equipment_from_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert!(world
			.get::<PendingEquipmentHold>(enrolled)
			.and_then(PendingEquipmentHold::inventory)
			.is_some_and(|inventory| matches!(
				inventory.primary_weapon(),
				Some(InventoryItem::Fisticuffs)
			)));
		assert_eq!(
			world.get::<EquippedItemHandler>(enrolled),
			Some(&EquippedItemHandler::Fisticuffs)
		);
		assert!(world.get::<PendingEquipmentTeardown>(enrolled).is_some());
		assert!(world.get::<PendingEquipmentHold>(enrolled).is_some());
		assert_eq!(
			world.get::<EquippedItemHandler>(unenrolled),
			Some(&EquippedItemHandler::Firearm)
		);
		assert!(world.get::<PendingEquipmentTeardown>(unenrolled).is_none());
		Ok(())
	}

	#[test]
	fn missing_bag_maps_to_none_and_pending_requests_are_not_overwritten() -> anyhow::Result<()> {
		let mut world = World::new();
		let actor = world
			.spawn((EquippedItemHandler::Grenade, ReconcileEquipmentFromInventory))
			.id();
		world
			.run_system_once(reconcile_equipment_from_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert_eq!(world.get::<EquippedItemHandler>(actor), Some(&EquippedItemHandler::None));
		assert!(world
			.get::<PendingEquipmentHold>(actor)
			.and_then(PendingEquipmentHold::inventory)
			.is_some_and(|inventory| inventory.primary_weapon().is_none()));

		world
			.entity_mut(actor)
			.remove::<(PendingEquipmentHold, PendingEquipmentTeardown)>();
		world.entity_mut(actor).insert((
			EquippedItemHandler::Firearm,
			PendingEquipmentHold::new(inventory(InventoryItem::firearm(FirearmMesh::Reltor))),
			PendingEquipmentTeardown,
		));
		world
			.run_system_once(reconcile_equipment_from_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert_eq!(world.get::<EquippedItemHandler>(actor), Some(&EquippedItemHandler::Firearm));
		assert_eq!(
			world
				.get::<PendingEquipmentHold>(actor)
				.and_then(PendingEquipmentHold::inventory)
				.and_then(Inventory::primary_weapon)
				.and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		Ok(())
	}

	#[test]
	fn unchanged_selection_and_removed_inventory_user_have_expected_results() -> anyhow::Result<()>
	{
		let mut world = World::new();
		let bag = world.spawn(inventory(InventoryItem::firearm(FirearmMesh::Bullpup))).id();
		let stable = world
			.spawn((
				InventoryUser::carrying(bag),
				EquippedItemHandler::Firearm,
				ReconcileEquipmentFromInventory,
			))
			.id();
		world
			.run_system_once(reconcile_equipment_from_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert!(world.get::<PendingEquipmentHold>(stable).is_none());
		assert!(world.get::<PendingEquipmentTeardown>(stable).is_none());

		let removed = world
			.spawn((
				InventoryUser::carrying(bag),
				EquippedItemHandler::Firearm,
				ReconcileEquipmentFromInventory,
			))
			.id();
		world.entity_mut(removed).remove::<InventoryUser>();
		world
			.run_system_once(reconcile_equipment_from_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		assert_eq!(world.get::<EquippedItemHandler>(removed), Some(&EquippedItemHandler::None));
		assert!(world.get::<PendingEquipmentHold>(removed).is_some());
		Ok(())
	}
}
