//! Bikanwarda Lubikan charge rules ([#1136](https://github.com/ramate-io/maybraid/issues/1136)).
//!
//! Self gameplay does not depend on this crate. Register [`BikanwardaLubikansPlugin`]
//! when Ward / Enraged content is present.

mod affiliation;
mod combat;
mod fixtures;
mod identity;
mod occupancy;
mod plugin;
mod snapshot;

pub use affiliation::{BikanwardaVictimClass, HERD_GROUP, PACK_GROUP, RAIDER_GROUP};
pub use combat::{BikanwardaCombatSystems, BikanwardaCombatWeights};
pub use fixtures::{
	assert_bikanwarda_equipped, equip_enraged, equip_ward, spawn_ward_owner,
	unlock_bikanwarda_accounts,
};
pub use identity::{
	BIKANWARDA_ENRAGED_ACCOUNT, BIKANWARDA_ENRAGED_DEFINITION, BIKANWARDA_WARD_ACCOUNT,
	BIKANWARDA_WARD_DEFINITION, PLACEHOLDER_BIKANWARDA_ACTIVATION_COST,
	PLACEHOLDER_BIKANWARDA_CAPACITY, PLACEHOLDER_BIKANWARDA_MAX_STOCK, enraged_definition,
	is_bikanwarda_account, is_bikanwarda_definition, ward_definition,
};
pub use occupancy::{BIKANWARDA_FOREST_BANYAN_RULE, extend_occupancy_rules, forest_banyan_rule};
pub use plugin::BikanwardaLubikansPlugin;
pub use snapshot::BikanwardaVictimSnapshot;

#[cfg(test)]
mod integration_tests;
