//! Generation contract for world placement ([#1117](https://github.com/ramate-io/maybraid/issues/1117)).
//!
//! Defines the stable interface between boss generation (#1116) and world
//! placement. #1117 consumes this to place the boss near initial spawn.

use bevy::prelude::*;
use bikanwarda_character::{BikanwardaActorRecipe, BikanwardaForm};

/// Boss actor recipe for character assembly.
///
/// Wraps the character recipe with form selection. Future generated bosses
/// can vary form/scale procedurally.
#[derive(Clone, Debug, PartialEq)]
pub struct BossActorRecipe {
	/// Boss form (Encounter, Ward, or Enraged).
	pub form: BikanwardaForm,
	
	/// Character assembly recipe.
	pub recipe: BikanwardaActorRecipe,
}

/// Boss attack configuration placeholder.
///
/// Future work: integrate with bikanwarda-attacks crate for actual attack
/// definitions. Scripted and generated bosses share the runtime attack system.
#[derive(Clone, Debug, PartialEq)]
pub struct BossAttackConfig {
	/// Placeholder until attack integration.
	pub placeholder: String,
}

/// Complete boss encounter contract for world placement.
///
/// Generated HCSG values provide this contract; presentation and world
/// placement consume it. Mutable runtime state (health, phase, unlocks)
/// lives outside this Arc.
#[derive(Clone, Debug, PartialEq)]
pub struct BossEncounterContract {
	/// Actor assembly configuration.
	pub actor: BossActorRecipe,
	
	/// Attack configuration.
	pub attacks: BossAttackConfig,
}

impl BossEncounterContract {
	/// The authored chained encounter configuration.
	pub fn chained() -> Self {
		Self {
			actor: BossActorRecipe {
				form: BikanwardaForm::Encounter,
				recipe: BikanwardaActorRecipe::for_form(BikanwardaForm::Encounter),
			},
			attacks: BossAttackConfig {
				placeholder: "chained encounter attacks".to_string(),
			},
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn chained_contract_is_stable() {
		let a = BossEncounterContract::chained();
		let b = BossEncounterContract::chained();
		assert_eq!(a.actor.form, b.actor.form);
		assert_eq!(a.actor.form, BikanwardaForm::Encounter);
	}
}
