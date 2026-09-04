use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};
use evasion_intelligence::{AssailantSource, EvasionIntelligenceUser};
use threat_intelligence::{
	AffiliationStrength, Affiliations, ThreatGroupId, ThreatId, ThreatKnowledge, ThreatRecord,
	ThreatSource,
};

use crate::{
	meets_commitment, proximity, select_tactic, CombatSelected, EvadeSelected, TacticScores,
	ThreatManagementElement, ThreatManagementIntelligence, ThreatManagementPlugin, ThreatTactic,
};

const FFA: ThreatGroupId = ThreatGroupId::group(1);

fn ffa_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(FFA, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(FFA, AffiliationStrength::permanent(1.0));
	affiliations
}

fn known_threat(entity: Entity, position: Vec3) -> ThreatKnowledge {
	let id = ThreatId(entity.to_bits());
	let record =
		ThreatRecord { id, entity, position, salience: 1.0, affiliations: ffa_affiliations(id) };
	let recipient = ffa_affiliations(ThreatId(1));
	let mut knowledge = ThreatKnowledge::default();
	assert!(knowledge
		.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2)
		.is_some());
	knowledge
}

#[test]
fn remaining_health_and_proximity_are_the_shared_axes() {
	let mode = ThreatManagementElement::new(2.0, 0.5);
	assert!((mode.score(0.5, 0.2) - 1.1).abs() < 1e-5);
	assert!((proximity(None, 80.0) - 0.0).abs() < 1e-5);
	assert!((proximity(Some(80.0), 80.0) - 0.5).abs() < 1e-5);
}

#[test]
fn empty_knowledge_forces_ignore_even_when_committed() {
	let scores = TacticScores { ignore: 0.0, evade: 8.0, combat: 8.0 };
	assert_eq!(
		select_tactic(true, ThreatTactic::Combat, true, true, scores, (1.0, 0.0)),
		ThreatTactic::Ignore
	);
}

#[test]
fn ffa_leaves_ignore_for_combat_while_threats_remain() {
	let management = ThreatManagementIntelligence::ffa();
	let scores = management.scores(1.0, 0.5);
	assert_eq!(
		select_tactic(false, ThreatTactic::Ignore, true, false, scores, management.commitment),
		ThreatTactic::Combat
	);
}

#[test]
fn committed_combat_does_not_yield_to_a_higher_evade_score() {
	let scores = TacticScores { ignore: 0.0, evade: 10.0, combat: 1.0 };
	assert_eq!(
		select_tactic(false, ThreatTactic::Combat, true, true, scores, (1.0, 0.0)),
		ThreatTactic::Combat
	);
	assert!(!meets_commitment(10.0, 1.0, (1.0, 0.0)));
}

#[test]
fn greedy_commitment_switches_to_the_better_challenger() {
	let scores = TacticScores { ignore: 0.0, evade: 2.0, combat: 1.0 };
	assert_eq!(
		select_tactic(false, ThreatTactic::Combat, true, true, scores, (1.0, 1.0)),
		ThreatTactic::Evade
	);
}

#[test]
fn zero_coefficients_stay_on_ignore() {
	let scores = TacticScores { ignore: 0.0, evade: 0.0, combat: 0.0 };
	assert_eq!(
		select_tactic(false, ThreatTactic::Ignore, true, true, scores, (1.0, 1.0)),
		ThreatTactic::Ignore
	);
}

#[test]
fn unavailable_combat_is_skipped() {
	let management = ThreatManagementIntelligence::ffa();
	let scores = management.scores(1.0, 1.0);
	assert_eq!(
		select_tactic(false, ThreatTactic::Ignore, false, false, scores, management.commitment),
		ThreatTactic::Ignore
	);
}

#[test]
fn select_grants_enemyship_and_tactic_markers() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, ThreatManagementPlugin));
	let threat = app.world_mut().spawn_empty().id();
	let knowledge = known_threat(threat, Vec3::X);
	let combatant = app
		.world_mut()
		.spawn((
			GlobalTransform::default(),
			knowledge.clone(),
			ThreatManagementIntelligence::ffa(),
			CombatTargeting::default(),
		))
		.id();
	let civilian = app
		.world_mut()
		.spawn((
			GlobalTransform::default(),
			knowledge,
			ThreatManagementIntelligence::civilian(),
			EvasionIntelligenceUser::default(),
		))
		.id();

	app.update();

	assert_eq!(
		app.world()
			.get::<ThreatManagementIntelligence>(combatant)
			.map(|user| user.tactic),
		Some(ThreatTactic::Combat)
	);
	assert!(app.world().get::<CombatSelected>(combatant).is_some());
	assert!(app.world().get::<CombatTargeting>(combatant).is_some_and(|targeting| {
		targeting.enabled
			&& targeting
				.active_target(threat)
				.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP))
	}));

	assert_eq!(
		app.world()
			.get::<ThreatManagementIntelligence>(civilian)
			.map(|user| user.tactic),
		Some(ThreatTactic::Evade)
	);
	assert!(app.world().get::<EvadeSelected>(civilian).is_some());
	assert!(app.world().get::<EvasionIntelligenceUser>(civilian).is_some_and(|evasion| {
		evasion.enabled
			&& evasion
				.active_assailant(threat)
				.is_some_and(|assailant| assailant.has_source(AssailantSource::ENEMYSHIP))
	}));
	Ok(())
}

#[test]
fn empty_knowledge_retracts_combat_membership() {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, ThreatManagementPlugin));
	let threat = app.world_mut().spawn_empty().id();
	let combatant = app
		.world_mut()
		.spawn((
			GlobalTransform::default(),
			ThreatKnowledge::default(),
			ThreatManagementIntelligence {
				tactic: ThreatTactic::Combat,
				generation: 1,
				..ThreatManagementIntelligence::ffa()
			},
			CombatTargeting::default(),
			CombatSelected,
		))
		.id();
	app.world_mut()
		.get_mut::<CombatTargeting>(combatant)
		.unwrap()
		.include(threat, TargetSource::ENEMYSHIP);
	app.update();
	let targeting = app.world().get::<CombatTargeting>(combatant).unwrap();
	assert!(!targeting.enabled);
	assert!(targeting.active_target(threat).is_none());
	assert_eq!(
		app.world()
			.get::<ThreatManagementIntelligence>(combatant)
			.map(|user| user.tactic),
		Some(ThreatTactic::Ignore)
	);
	assert!(app.world().get::<CombatSelected>(combatant).is_none());
}
