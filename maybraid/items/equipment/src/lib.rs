//! Actor equipment ownership: category, holster commit, and equipped dispatch.
//!
//! Character produces actor-addressed control frames and semantic intents.
//! This crate resolves the installed [`EquippedItemHandler`] and requests kit
//! changes. Firearm / grenade crates realize construction and teardown.

mod dispatch;
mod lifecycle;
mod selection;
mod transition;

use bevy::ecs::schedule::ApplyDeferred;
use bevy::prelude::*;
use maybraid_character_controller::{CharacterControlSystems, CharacterControllerPlugin};

pub use dispatch::{
	dispatch_equipped_item, dispatch_equipped_power, EquippedItemHandler, EquippedItemUse,
	EquippedPowerUse, RoutedItemActor,
};
pub use lifecycle::{
	clear_equipment_requests, EquipmentInventoryChanged, PendingEquipmentHold,
	PendingEquipmentTeardown,
};
pub use maybraid_character_controller::InventoryCyclingCategory;
pub use selection::{
	apply_inventory_category_selection, cycle_equipped_inventory_queues,
	ActiveInventoryCyclingCategory, EquippedLubikanQueue, InventoryCycling,
};
pub use transition::{
	advance_equipment_holster, begin_equipment_holster, clear_finished_equipment_holsters,
	commit_equipment_holster, BlocksEquipmentSwap, EquipmentHolster, EQUIPMENT_HOLSTER_SECS,
};

/// Domain schedule points hosts order against.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EquipmentSystems {
	/// D-pad category pick. Runs before holster begin and queue cycling.
	SelectCategory,
	/// Start a weapon holster when the weapons category is selected.
	BeginHolster,
	/// Cycle skill-map / Lubikan queues. Weapons return without mutating.
	CycleQueue,
	/// Advance in-flight holster clocks.
	AdvanceHolster,
	/// Commit the bag and request kit replace at the holster midpoint.
	CommitHolster,
	/// Item crates tear down the outgoing kit.
	Teardown,
	/// Item crates construct the incoming kit after teardown commands flush.
	Hold,
	/// Drop realized hold / teardown request components.
	Cleanup,
}

/// Registers equipment contracts, selection, holster, and Bind dispatch.
pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<CharacterControllerPlugin>() {
			app.add_plugins(CharacterControllerPlugin);
		}
		app.add_message::<EquipmentInventoryChanged>()
			.init_resource::<ActiveInventoryCyclingCategory>()
			.configure_sets(
				Update,
				(
					EquipmentSystems::SelectCategory.after(CharacterControlSystems),
					EquipmentSystems::BeginHolster
						.after(EquipmentSystems::SelectCategory)
						.before(EquipmentSystems::CycleQueue)
						.after(CharacterControlSystems),
					EquipmentSystems::CycleQueue
						.after(EquipmentSystems::SelectCategory)
						.after(CharacterControlSystems),
					EquipmentSystems::AdvanceHolster,
					EquipmentSystems::CommitHolster.after(EquipmentSystems::AdvanceHolster),
					EquipmentSystems::Teardown.after(EquipmentSystems::CommitHolster),
					EquipmentSystems::Hold.after(EquipmentSystems::Teardown),
					EquipmentSystems::Cleanup.after(EquipmentSystems::Hold),
				),
			)
			.add_systems(
				Update,
				ApplyDeferred.after(EquipmentSystems::Teardown).before(EquipmentSystems::Hold),
			)
			.add_observer(dispatch_equipped_item)
			.add_observer(dispatch_equipped_power)
			.add_systems(
				Update,
				apply_inventory_category_selection.in_set(EquipmentSystems::SelectCategory),
			)
			.add_systems(Update, begin_equipment_holster.in_set(EquipmentSystems::BeginHolster))
			.add_systems(
				Update,
				cycle_equipped_inventory_queues.in_set(EquipmentSystems::CycleQueue),
			)
			.add_systems(
				Update,
				(
					advance_equipment_holster,
					clear_finished_equipment_holsters.after(advance_equipment_holster),
				)
					.in_set(EquipmentSystems::AdvanceHolster),
			)
			.add_systems(Update, commit_equipment_holster.in_set(EquipmentSystems::CommitHolster))
			.add_systems(Update, clear_equipment_requests.in_set(EquipmentSystems::Cleanup));
	}
}
