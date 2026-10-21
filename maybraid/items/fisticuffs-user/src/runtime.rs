//! Equipment realize: hold / teardown the fisticuffs driver.

use bevy::prelude::*;
use character_items::InventoryItem;
use maybraid_equipment::{
	EquipmentSystems, EquippedItemHandler, PendingEquipmentHold, PendingEquipmentTeardown,
};

use crate::FisticuffsUser;

#[derive(Resource, Default)]
struct FisticuffsUserRuntime;

/// Attack machine + equipment realize. Gameplay assembly is the owner that
/// calls this; a second call is a no-op.
pub fn register_fisticuffs_user_runtime(app: &mut App) {
	if app.world().contains_resource::<FisticuffsUserRuntime>() {
		return;
	}
	app.init_resource::<FisticuffsUserRuntime>()
		.add_message::<maybraid_equipment::EquippedItemUse>()
		.add_systems(
			Update,
			realize_equipment_teardown
				.in_set(EquipmentSystems::Teardown)
				.before(player::PlayerSystems::Intent),
		)
		.add_systems(Update, realize_equipment_hold.in_set(EquipmentSystems::Hold));
}

/// Cancel the in-flight jab and drop the driver. Sensors follow
/// [`RemovedComponents<FisticuffsUser>`] in [`crate::sync_fist_sensors`].
pub fn teardown_fisticuffs(commands: &mut Commands, user: Entity) {
	if let Ok(mut entity) = commands.get_entity(user) {
		entity.remove::<FisticuffsUser>();
	}
}

fn realize_equipment_teardown(
	mut commands: Commands,
	mut pending: Query<(Entity, Option<&mut FisticuffsUser>), With<PendingEquipmentTeardown>>,
) {
	for (entity, user) in &mut pending {
		if let Some(mut user) = user {
			user.state.cancel();
		}
		teardown_fisticuffs(&mut commands, entity);
	}
}

fn realize_equipment_hold(mut commands: Commands, pending: Query<(Entity, &PendingEquipmentHold)>) {
	for (entity, hold) in &pending {
		if !matches!(hold.inventory().primary_weapon(), Some(InventoryItem::Fisticuffs)) {
			continue;
		}
		if let Ok(mut entity) = commands.get_entity(entity) {
			entity.try_insert(FisticuffsUser::default());
			EquippedItemHandler::Fisticuffs.install(&mut entity);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::register_fisticuffs_user_runtime;
	use bevy::prelude::*;

	#[test]
	fn runtime_register_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_fisticuffs_user_runtime(&mut app);
		register_fisticuffs_user_runtime(&mut app);
	}
}
