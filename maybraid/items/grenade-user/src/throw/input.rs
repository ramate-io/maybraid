//! Addressed use → cook start.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use firearm_user::WeaponSwap;
use maybraid_equipment::EquippedItemUse;

use super::{GrenadeThrow, GrenadeUser, ThrowJingleDue};

pub fn apply_throw_intents(
	use_item: On<EquippedItemUse>,
	bags: Query<&Inventory>,
	carriers: Query<&InventoryUser>,
	mut commands: Commands,
	mut users: Query<(&mut GrenadeThrow, Has<WeaponSwap>), With<GrenadeUser>>,
) {
	if !use_item.handler().is_grenade() {
		return;
	}
	let entity = use_item.entity;
	let Ok((mut throw, swapping)) = users.get_mut(entity) else {
		return;
	};
	throw.apply_use(use_item.held());
	if !use_item.held() {
		return;
	}
	let ready = carriers
		.get(entity)
		.ok()
		.and_then(|carrier| bags.get(carrier.bag).ok())
		.and_then(Inventory::primary_weapon)
		.is_some_and(|item| match item {
			InventoryItem::Grenade { recharge, .. } => recharge.ready(),
			_ => false,
		});
	let fuse = carriers
		.get(entity)
		.ok()
		.and_then(|carrier| bags.get(carrier.bag).ok())
		.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
		.map(|stats| stats.fuse)
		.unwrap_or_else(|| GrenadeStats::standard().fuse);
	if throw.try_start_cook(ready, swapping, fuse) {
		commands.entity(entity).try_insert(ThrowJingleDue);
	}
}
