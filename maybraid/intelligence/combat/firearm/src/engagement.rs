//! Per-combatant weapons grant. Classification, spotting, and aim stay live
//! while this is [`RulesOfEngagement::Hold`].

use std::collections::BTreeSet;

use bevy::prelude::*;
use damage::DamageApplied;

/// Whether this combatant may currently pull the trigger, and on whom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RulesOfEngagement {
	/// Rank and aim, but never fire.
	Hold,
	/// Fire only at entities that have provoked this combatant.
	ReturnFire,
	/// Fire at the current engaged hostile.
	#[default]
	WeaponsFree,
}

/// Installed rules of engagement for one firearm combatant.
///
/// Absence of this component is treated as [`RulesOfEngagement::WeaponsFree`].
#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct FirearmEngagement {
	pub rules: RulesOfEngagement,
	authorized: BTreeSet<Entity>,
}

impl Default for FirearmEngagement {
	fn default() -> Self {
		Self::weapons_free()
	}
}

impl FirearmEngagement {
	pub fn hold() -> Self {
		Self { rules: RulesOfEngagement::Hold, authorized: BTreeSet::new() }
	}

	pub fn return_fire() -> Self {
		Self { rules: RulesOfEngagement::ReturnFire, authorized: BTreeSet::new() }
	}

	pub fn weapons_free() -> Self {
		Self { rules: RulesOfEngagement::WeaponsFree, authorized: BTreeSet::new() }
	}

	pub fn set_rules(&mut self, rules: RulesOfEngagement) {
		self.rules = rules;
	}

	pub fn authorize(&mut self, subject: Entity) {
		self.authorized.insert(subject);
	}

	/// Record an attacker under [`RulesOfEngagement::ReturnFire`].
	pub fn note_provocation(&mut self, subject: Entity) {
		if self.rules == RulesOfEngagement::ReturnFire {
			self.authorize(subject);
		}
	}

	pub fn may_fire_at(&self, subject: Entity) -> bool {
		match self.rules {
			RulesOfEngagement::Hold => false,
			RulesOfEngagement::ReturnFire => self.authorized.contains(&subject),
			RulesOfEngagement::WeaponsFree => true,
		}
	}
}

pub fn allows_fire(engagement: Option<&FirearmEngagement>, subject: Entity) -> bool {
	engagement.is_none_or(|engagement| engagement.may_fire_at(subject))
}

/// Received damage authorizes the attacker only while return-fire is in force.
pub fn authorize_return_fire_from_damage(
	mut applied: MessageReader<DamageApplied>,
	mut combatants: Query<&mut FirearmEngagement>,
) {
	for event in applied.read() {
		let Some(source) = event.source else {
			continue;
		};
		if source == event.target {
			continue;
		}
		let Ok(mut engagement) = combatants.get_mut(event.target) else {
			continue;
		};
		engagement.note_provocation(source);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hold_never_authorizes_a_shot() {
		let mut engagement = FirearmEngagement::hold();
		let attacker = Entity::from_bits(2);
		engagement.note_provocation(attacker);
		assert!(!engagement.may_fire_at(attacker));
	}

	#[test]
	fn return_fire_only_authorizes_the_attacker() {
		let mut engagement = FirearmEngagement::return_fire();
		let attacker = Entity::from_bits(2);
		let bystander = Entity::from_bits(3);
		engagement.note_provocation(attacker);
		assert!(engagement.may_fire_at(attacker));
		assert!(!engagement.may_fire_at(bystander));
	}

	#[test]
	fn weapons_free_does_not_need_authorization() {
		let engagement = FirearmEngagement::weapons_free();
		assert!(engagement.may_fire_at(Entity::from_bits(2)));
		assert!(allows_fire(None, Entity::from_bits(2)));
	}
}
