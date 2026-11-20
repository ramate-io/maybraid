//! Bikanwarda catalog ids and fixture accounts.

use lubikans::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};

/// Shared PLACEHOLDER balance row for charge, unlock and summon consumers.
pub const PLACEHOLDER_BIKANWARDA_CAPACITY: f32 = 100.0;
pub const PLACEHOLDER_BIKANWARDA_ACTIVATION_COST: f32 = 25.0;
pub const PLACEHOLDER_BIKANWARDA_MAX_STOCK: u32 = 4;

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
	LubikanDefinition::with_stock(
		BIKANWARDA_WARD_DEFINITION,
		PLACEHOLDER_BIKANWARDA_CAPACITY,
		PLACEHOLDER_BIKANWARDA_ACTIVATION_COST,
		PLACEHOLDER_BIKANWARDA_MAX_STOCK,
	)
}

/// Default Enraged row. Charge rules match Ward until product decides otherwise.
pub fn enraged_definition() -> LubikanDefinition {
	LubikanDefinition::with_stock(
		BIKANWARDA_ENRAGED_DEFINITION,
		PLACEHOLDER_BIKANWARDA_CAPACITY,
		PLACEHOLDER_BIKANWARDA_ACTIVATION_COST,
		PLACEHOLDER_BIKANWARDA_MAX_STOCK,
	)
}
