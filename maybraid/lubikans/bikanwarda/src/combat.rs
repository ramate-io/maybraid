//! Bikanwarda combat → charge. One [`DamageApplied`] may write hit and kill requests.

use bevy::prelude::*;
use damage::{DamageApplied, DamageSystems};
use lubikans::{
	LubikanAwardPhase, LubikanChargeReason, LubikanChargeRequest, LubikanLedger, LubikanOwner,
};
use player::Player;
use threat_intelligence::Affiliations;

use crate::affiliation::BikanwardaVictimClass;
use crate::identity::is_bikanwarda_account;
use crate::snapshot::BikanwardaVictimSnapshot;

/// Fixed combat weights ([#1136](https://github.com/ramate-io/maybraid/issues/1136)).
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct BikanwardaCombatWeights {
	pub animal_hit: f32,
	pub animal_kill: f32,
	pub raider_hit: f32,
	pub raider_kill: f32,
	/// When true, a lethal hit writes both hit and kill credits.
	pub stack_kill: bool,
}

impl Default for BikanwardaCombatWeights {
	fn default() -> Self {
		Self {
			animal_hit: -5.0,
			animal_kill: -10.0,
			raider_hit: 5.0,
			raider_kill: 25.0,
			stack_kill: true,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct CombatCredits {
	hit: Option<f32>,
	kill: Option<f32>,
}

impl BikanwardaCombatWeights {
	pub fn credits_for(
		self,
		class: BikanwardaVictimClass,
		hit: DamageApplied,
	) -> CombatCredits {
		if hit.is_zero_effect() || class == BikanwardaVictimClass::Neutral {
			return CombatCredits::default();
		}
		let lethal = hit.remaining <= 0.0;
		let (hit_w, kill_w) = match class {
			BikanwardaVictimClass::Animal => (self.animal_hit, self.animal_kill),
			BikanwardaVictimClass::Raider => (self.raider_hit, self.raider_kill),
			BikanwardaVictimClass::Neutral => return CombatCredits::default(),
		};
		let hit_credit = if !lethal || self.stack_kill {
			Some(hit_w).filter(|value| *value != 0.0)
		} else {
			None
		};
		let kill_credit =
			if lethal { Some(kill_w).filter(|value| *value != 0.0) } else { None };
		CombatCredits { hit: hit_credit, kill: kill_credit }
	}
}

pub(crate) fn configure(app: &mut App) {
	if app.world().get_resource::<Messages<DamageApplied>>().is_none() {
		app.add_message::<DamageApplied>();
	}
	app.init_resource::<BikanwardaCombatWeights>().add_systems(
		PostUpdate,
		award_bikanwarda_combat
			.in_set(LubikanAwardPhase::Credit)
			.after(DamageSystems::Apply),
	);
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BikanwardaCombatSystems {
	Snapshot,
}

fn resolve_victim_class(
	target: Entity,
	affiliations: &Query<&Affiliations>,
	snapshots: &Query<&BikanwardaVictimSnapshot>,
) -> BikanwardaVictimClass {
	if let Ok(snapshot) = snapshots.get(target) {
		return snapshot.class;
	}
	affiliations
		.get(target)
		.map(BikanwardaVictimClass::from_affiliations)
		.unwrap_or(BikanwardaVictimClass::Neutral)
}

type LiveOwner = Or<(With<Player>, With<LubikanOwner>)>;

fn award_bikanwarda_combat(
	mut hits: MessageReader<DamageApplied>,
	mut requests: MessageWriter<LubikanChargeRequest>,
	weights: Res<BikanwardaCombatWeights>,
	owners: Query<Entity, LiveOwner>,
	ledgers: Query<&LubikanLedger, LiveOwner>,
	affiliations: Query<&Affiliations>,
	snapshots: Query<&BikanwardaVictimSnapshot>,
	mut commands: Commands,
) {
	for hit in hits.read() {
		let Some(source) = hit.source else {
			continue;
		};
		if owners.get(source).is_err() {
			continue;
		}
		if owners.get(hit.target).is_ok() {
			continue;
		}
		let Ok(ledger) = ledgers.get(source) else {
			continue;
		};
		let equipped = ledger.selection().resolved();
		if !is_bikanwarda_account(equipped) {
			continue;
		}
		let class = resolve_victim_class(hit.target, &affiliations, &snapshots);
		if snapshots.get(hit.target).is_err() && class != BikanwardaVictimClass::Neutral {
			commands.entity(hit.target).insert(BikanwardaVictimSnapshot { class });
		}
		let credits = weights.credits_for(class, *hit);
		if let Some(requested) = credits.hit {
			requests.write(LubikanChargeRequest::new(
				source,
				Some(equipped),
				requested,
				LubikanChargeReason::COMBAT_HIT,
				Some(hit.point),
			));
		}
		if let Some(requested) = credits.kill {
			requests.write(LubikanChargeRequest::new(
				source,
				Some(equipped),
				requested,
				LubikanChargeReason::COMBAT_KILL,
				Some(hit.point),
			));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::affiliation::{HERD_GROUP, PACK_GROUP, RAIDER_GROUP};
	use threat_intelligence::{AffiliationStrength, Affiliations, ThreatGroupId};

	fn applied(remaining: f32) -> DamageApplied {
		DamageApplied {
			target: Entity::from_bits(2),
			source: Some(Entity::from_bits(1)),
			amount: 10.0,
			applied: 10.0,
			remaining,
			point: Vec3::ZERO,
		}
	}

	fn affiliations_with(group: ThreatGroupId) -> Affiliations {
		let mut affiliations = Affiliations::default();
		affiliations.join(group, AffiliationStrength::permanent(1.0));
		affiliations
	}

	#[test]
	fn weights_match_issue_defaults() {
		let w = BikanwardaCombatWeights::default();
		assert_eq!(w.animal_hit, -5.0);
		assert_eq!(w.animal_kill, -10.0);
		assert_eq!(w.raider_hit, 5.0);
		assert_eq!(w.raider_kill, 25.0);
	}

	#[test]
	fn lethal_animal_stacks_hit_and_kill_by_default() {
		let w = BikanwardaCombatWeights::default();
		let credits = w.credits_for(BikanwardaVictimClass::Animal, applied(0.0));
		assert_eq!(credits.hit, Some(-5.0));
		assert_eq!(credits.kill, Some(-10.0));
	}

	#[test]
	fn lethal_raider_stacks_positive_credits() {
		let w = BikanwardaCombatWeights::default();
		let credits = w.credits_for(BikanwardaVictimClass::Raider, applied(0.0));
		assert_eq!(credits.hit, Some(5.0));
		assert_eq!(credits.kill, Some(25.0));
	}

	#[test]
	fn zero_effect_awards_nothing() {
		let w = BikanwardaCombatWeights::default();
		let hit = DamageApplied {
			target: Entity::from_bits(2),
			source: Some(Entity::from_bits(1)),
			amount: 5.0,
			applied: 0.0,
			remaining: 10.0,
			point: Vec3::ZERO,
		};
		let credits = w.credits_for(BikanwardaVictimClass::Raider, hit);
		assert_eq!(credits, CombatCredits::default());
	}

	#[test]
	fn classifies_herd_pack_and_raider_groups() {
		assert_eq!(
			BikanwardaVictimClass::from_affiliations(&affiliations_with(HERD_GROUP)),
			BikanwardaVictimClass::Animal
		);
		assert_eq!(
			BikanwardaVictimClass::from_affiliations(&affiliations_with(PACK_GROUP)),
			BikanwardaVictimClass::Animal
		);
		assert_eq!(
			BikanwardaVictimClass::from_affiliations(&affiliations_with(RAIDER_GROUP)),
			BikanwardaVictimClass::Raider
		);
	}
}
