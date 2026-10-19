//! Authored Bikanwarda identity and Lubikan catalog ids.

use encounters::EncounterId;
use lubikans::{LubikanDefinition, LubikanDefinitionId};

/// Stable slug. Future HCSG hosts should reuse this as `Id::Bytes`.
pub const BIKANWARDA_CHAINED_SLUG: &str = "bikanwarda.chained.v0";

/// Catalog id for Enraged. Content crates start at 1; Self is 0.
pub const ENRAGED_DEFINITION_ID: LubikanDefinitionId = LubikanDefinitionId::from_raw(1);

/// Catalog id for Ward.
pub const WARD_DEFINITION_ID: LubikanDefinitionId = LubikanDefinitionId::from_raw(2);

/// PLACEHOLDER Enraged capacity. Balancing should change [`crate::BikanwardaUnlockConfig`].
pub const PLACEHOLDER_ENRAGED_CAPACITY: f32 = 512.0;

/// PLACEHOLDER Enraged activation cost.
pub const PLACEHOLDER_ENRAGED_COST: f32 = 512.0;

/// PLACEHOLDER Ward capacity.
pub const PLACEHOLDER_WARD_CAPACITY: f32 = 512.0;

/// PLACEHOLDER Ward activation cost.
pub const PLACEHOLDER_WARD_COST: f32 = 512.0;

pub fn chained_encounter_id() -> EncounterId {
	EncounterId::from_slug(BIKANWARDA_CHAINED_SLUG)
}

pub fn enraged_definition(capacity: f32, cost: f32) -> LubikanDefinition {
	LubikanDefinition::new(ENRAGED_DEFINITION_ID, capacity, cost)
}

pub fn ward_definition(capacity: f32, cost: f32) -> LubikanDefinition {
	LubikanDefinition::new(WARD_DEFINITION_ID, capacity, cost)
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
