//! Bikanwarda chained-encounter outcomes and summon unlocks.
//!
//! [#1121](https://github.com/ramate-io/maybraid/issues/1121). Sleep is
//! [#1120](https://github.com/ramate-io/maybraid/issues/1120). Generation and
//! world placement are out of scope.

mod identity;
mod outcomes;
mod plugin;
mod unlocks;

pub use identity::{
	chained_encounter_id, enraged_definition, ward_definition, BIKANWARDA_CHAINED_SLUG,
	ENRAGED_DEFINITION_ID, PLACEHOLDER_ENRAGED_CAPACITY, PLACEHOLDER_ENRAGED_COST,
	PLACEHOLDER_WARD_CAPACITY, PLACEHOLDER_WARD_COST, WARD_DEFINITION_ID,
};
pub use outcomes::{BikanwardaDeparted, BikanwardaDeparture, EncounterUnlockAwarded};
pub use plugin::BikanwardaEncounterPlugin;
pub use unlocks::{register_unlocks, BikanwardaUnlockConfig};

#[cfg(test)]
mod tests;
