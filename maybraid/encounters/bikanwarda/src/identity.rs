//! Authored Bikanwarda identity and Lubikan catalog ids.

use encounters::EncounterId;
use lubikans::LubikanDefinition;

/// Stable slug. Future HCSG hosts should reuse this as `Id::Bytes`.
pub const BIKANWARDA_CHAINED_SLUG: &str = "bikanwarda.chained.v0";

// Charge and unlock consumers share identity and PLACEHOLDER balance values.
pub use bikanwarda_lubikans::{
	BIKANWARDA_ENRAGED_DEFINITION as ENRAGED_DEFINITION_ID,
	BIKANWARDA_WARD_DEFINITION as WARD_DEFINITION_ID,
	PLACEHOLDER_BIKANWARDA_ACTIVATION_COST as PLACEHOLDER_ENRAGED_COST,
	PLACEHOLDER_BIKANWARDA_ACTIVATION_COST as PLACEHOLDER_WARD_COST,
	PLACEHOLDER_BIKANWARDA_CAPACITY as PLACEHOLDER_ENRAGED_CAPACITY,
	PLACEHOLDER_BIKANWARDA_CAPACITY as PLACEHOLDER_WARD_CAPACITY,
};

pub fn chained_encounter_id() -> EncounterId {
	EncounterId::from_slug(BIKANWARDA_CHAINED_SLUG)
}

pub fn enraged_definition(capacity: f32, cost: f32) -> LubikanDefinition {
	LubikanDefinition::with_stock(
		ENRAGED_DEFINITION_ID,
		capacity,
		cost,
		bikanwarda_lubikans::PLACEHOLDER_BIKANWARDA_MAX_STOCK,
	)
}

pub fn ward_definition(capacity: f32, cost: f32) -> LubikanDefinition {
	LubikanDefinition::with_stock(
		WARD_DEFINITION_ID,
		capacity,
		cost,
		bikanwarda_lubikans::PLACEHOLDER_BIKANWARDA_MAX_STOCK,
	)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn chained_id_is_stable() {
		assert_eq!(chained_encounter_id(), EncounterId::from_slug(BIKANWARDA_CHAINED_SLUG));
		assert_eq!(chained_encounter_id().slug_str(), Some(BIKANWARDA_CHAINED_SLUG));
		assert_ne!(ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID);
		assert!(!ENRAGED_DEFINITION_ID.is_self());
		assert!(!WARD_DEFINITION_ID.is_self());
	}
}
