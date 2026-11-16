//! Bikanwarda chained-encounter generation, outcomes, and summon unlocks.
//!
//! [#1116](https://github.com/ramate-io/maybraid/issues/1116) HCSG generation and Scripted authoring.
//! [#1121](https://github.com/ramate-io/maybraid/issues/1121) outcomes and unlocks.
//! [#1120](https://github.com/ramate-io/maybraid/issues/1120) sleep phase.
//! [#1117](https://github.com/ramate-io/maybraid/issues/1117) world placement (consumes generation contract).

mod assembly;
mod contract;
mod generation;
mod identity;
mod outcomes;
mod plugin;
mod presentation;
mod scripted;
mod unlocks;

pub use assembly::{spawn_boss_assemblies, BossAssemblyPlugin};
pub use contract::{BossActorRecipe, BossAttackConfig, BossEncounterContract};
pub use generation::BikanwardaBossEncounter;
pub use identity::{
	chained_encounter_id, enraged_definition, ward_definition, BIKANWARDA_CHAINED_SLUG,
	ENRAGED_DEFINITION_ID, PLACEHOLDER_ENRAGED_CAPACITY, PLACEHOLDER_ENRAGED_COST,
	PLACEHOLDER_WARD_CAPACITY, PLACEHOLDER_WARD_COST, WARD_DEFINITION_ID,
};
pub use outcomes::{BikanwardaDeparted, BikanwardaDeparture, EncounterUnlockAwarded};
pub use plugin::BikanwardaEncounterPlugin;
pub use presentation::{BossEncounterPresentationPlugin, BossEncounterRoot, RequestBossAssembly};
pub use scripted::Scripted;
pub use unlocks::{register_unlocks, BikanwardaUnlockConfig};

#[cfg(test)]
mod tests;
