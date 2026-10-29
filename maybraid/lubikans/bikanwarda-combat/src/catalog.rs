//! Ward / Enraged catalog ids and summon recipes.
//!
//! Distinct from the fixture placeholder in [`lubikans`]. Charge numbers copy
//! the labeled PLACEHOLDER summon defaults until [#1135](https://github.com/ramate-io/maybraid/issues/1135)
//! agrees them.

use lubikans::{
	LubikanCatalog, LubikanSummonCatalog, LubikanSummonPresentation, LubikanSummonRecipe,
	PLACEHOLDER_SUMMON_HEALTH, PLACEHOLDER_SUMMON_LIFETIME_SECS,
};

pub use bikanwarda_lubikans::{
	BIKANWARDA_ENRAGED_DEFINITION as ENRAGED_DEFINITION_ID,
	BIKANWARDA_WARD_DEFINITION as WARD_DEFINITION_ID, enraged_definition, ward_definition,
};

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
