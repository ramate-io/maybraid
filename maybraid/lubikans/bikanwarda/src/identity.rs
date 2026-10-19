//! Bikanwarda catalog ids and fixture accounts.

use lubikans::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};

/// Ward form catalog entry ([#1136](https://github.com/ramate-io/maybraid/issues/1136)).
pub const BIKANWARDA_WARD_DEFINITION: LubikanDefinitionId = LubikanDefinitionId::from_raw(1);
/// Enraged form catalog entry (summon lifecycle [#1135](https://github.com/ramate-io/maybraid/issues/1135)).
pub const BIKANWARDA_ENRAGED_DEFINITION: LubikanDefinitionId = LubikanDefinitionId::from_raw(2);

/// Instance-0 Ward account for one owner.
pub const BIKANWARDA_WARD_ACCOUNT: LubikanAccountId =
	LubikanAccountId::unique(BIKANWARDA_WARD_DEFINITION);
/// Instance-0 Enraged account for one owner.
pub const BIKANWARDA_ENRAGED_ACCOUNT: LubikanAccountId =
	LubikanAccountId::unique(BIKANWARDA_ENRAGED_DEFINITION);

pub fn is_bikanwarda_definition(id: LubikanDefinitionId) -> bool {
	id == BIKANWARDA_WARD_DEFINITION || id == BIKANWARDA_ENRAGED_DEFINITION
}

pub fn is_bikanwarda_account(id: LubikanAccountId) -> bool {
	is_bikanwarda_definition(id.definition())
}

/// Default Ward row for fixtures and catalog registration.
pub fn ward_definition() -> LubikanDefinition {
	LubikanDefinition::with_stock(BIKANWARDA_WARD_DEFINITION, 100.0, 25.0, 4)
}

/// Default Enraged row. Charge rules match Ward until product decides otherwise.
pub fn enraged_definition() -> LubikanDefinition {
	LubikanDefinition::with_stock(BIKANWARDA_ENRAGED_DEFINITION, 100.0, 25.0, 4)
}
