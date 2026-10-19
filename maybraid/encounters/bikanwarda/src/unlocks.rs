//! Ward / Enraged catalog rows. Summon combat is [#1106](https://github.com/ramate-io/maybraid/issues/1106).

use bevy::prelude::*;
use lubikans::{LubikanCatalog, LubikanDefinition};

use crate::identity::{
	enraged_definition, ward_definition, PLACEHOLDER_ENRAGED_CAPACITY, PLACEHOLDER_ENRAGED_COST,
	PLACEHOLDER_WARD_CAPACITY, PLACEHOLDER_WARD_COST,
};

/// Configurable unlock limits. Not a closed balance decision.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct BikanwardaUnlockConfig {
	pub enraged_capacity: f32,
	pub enraged_cost: f32,
	pub ward_capacity: f32,
	pub ward_cost: f32,
}

impl Default for BikanwardaUnlockConfig {
	fn default() -> Self {
		Self {
			enraged_capacity: PLACEHOLDER_ENRAGED_CAPACITY,
			enraged_cost: PLACEHOLDER_ENRAGED_COST,
			ward_capacity: PLACEHOLDER_WARD_CAPACITY,
			ward_cost: PLACEHOLDER_WARD_COST,
		}
	}
}

impl BikanwardaUnlockConfig {
	pub fn enraged(self) -> LubikanDefinition {
		enraged_definition(self.enraged_capacity, self.enraged_cost)
	}

	pub fn ward(self) -> LubikanDefinition {
		ward_definition(self.ward_capacity, self.ward_cost)
	}
}

pub fn register_unlocks(catalog: &mut LubikanCatalog, config: &BikanwardaUnlockConfig) {
	catalog.register(config.enraged());
	catalog.register(config.ward());
}
