//! Ward / Enraged catalog ids and summon recipes.
//!
//! Distinct from the fixture placeholder in [`lubikans`]. Charge numbers copy
//! the labeled PLACEHOLDER summon defaults until [#1135](https://github.com/ramate-io/maybraid/issues/1135)
//! agrees them.

use lubikans::{
	LubikanCatalog, LubikanDefinition, LubikanDefinitionId, LubikanSummonCatalog,
	LubikanSummonPresentation, LubikanSummonRecipe, PLACEHOLDER_SUMMON_ACTIVATION_COST,
	PLACEHOLDER_SUMMON_CAPACITY, PLACEHOLDER_SUMMON_HEALTH, PLACEHOLDER_SUMMON_LIFETIME_SECS,
};

/// Ward catalog id. Not the fixture `0xFFFF_FF01` placeholder.
pub const WARD_DEFINITION_ID: LubikanDefinitionId = LubikanDefinitionId::from_raw(0xB1A0_0001);

/// Enraged catalog id. Not the fixture `0xFFFF_FF01` placeholder.
pub const ENRAGED_DEFINITION_ID: LubikanDefinitionId = LubikanDefinitionId::from_raw(0xB1A0_0002);

pub fn ward_definition() -> LubikanDefinition {
	LubikanDefinition::new(
		WARD_DEFINITION_ID,
		PLACEHOLDER_SUMMON_CAPACITY,
		PLACEHOLDER_SUMMON_ACTIVATION_COST,
	)
}

pub fn enraged_definition() -> LubikanDefinition {
	LubikanDefinition::new(
		ENRAGED_DEFINITION_ID,
		PLACEHOLDER_SUMMON_CAPACITY,
		PLACEHOLDER_SUMMON_ACTIVATION_COST,
	)
}

pub fn ward_recipe() -> LubikanSummonRecipe {
	LubikanSummonRecipe {
		definition: WARD_DEFINITION_ID,
		health: PLACEHOLDER_SUMMON_HEALTH,
		lifetime_secs: PLACEHOLDER_SUMMON_LIFETIME_SECS,
		presentation: LubikanSummonPresentation::PlaceholderCapsule,
	}
}

pub fn enraged_recipe() -> LubikanSummonRecipe {
	LubikanSummonRecipe {
		definition: ENRAGED_DEFINITION_ID,
		health: PLACEHOLDER_SUMMON_HEALTH,
		lifetime_secs: PLACEHOLDER_SUMMON_LIFETIME_SECS,
		presentation: LubikanSummonPresentation::PlaceholderCapsule,
	}
}

/// Register both forms on the existing Self catalog / summon recipe table.
pub fn register_bikanwarda_summons(
	catalog: &mut LubikanCatalog,
	recipes: &mut LubikanSummonCatalog,
) {
	catalog.register(ward_definition());
	catalog.register(enraged_definition());
	recipes.register(ward_recipe());
	recipes.register(enraged_recipe());
}
