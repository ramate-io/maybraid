use std::collections::HashSet;

use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};
use damage::Health;
use evasion_intelligence::{AssailantSource, EvasionIntelligenceUser};
use threat_intelligence::ThreatKnowledge;

use crate::{
	nearest_known_xz, proximity, select_tactic, CombatSelected, EvadeSelected,
	ThreatManagementIntelligence, ThreatTactic, ThreatTacticChanged,
};

type Managers<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static GlobalTransform,
		&'static ThreatKnowledge,
		&'static mut ThreatManagementIntelligence,
		Option<&'static Health>,
		Option<&'static mut CombatTargeting>,
		Option<&'static mut EvasionIntelligenceUser>,
	),
>;

/// Score retained threats and grant the matching combat or evasion actuator.
pub fn select_threat_tactics(
	time: Res<Time>,
	mut commands: Commands,
	mut changed: MessageWriter<ThreatTacticChanged>,
	mut managers: Managers,
) {
	let now = time.elapsed_secs();
	for (entity, transform, knowledge, mut management, health, mut targeting, mut evasion) in
		&mut managers
	{
		if now < management.next_select_at {
			continue;
		}
		management.next_select_at = now + management.selection_interval.max(0.0);

		let health_fraction = health.map(|health| health.fraction()).unwrap_or(1.0);
		let proximity = proximity(
			nearest_known_xz(knowledge, transform.translation()),
			management.proximity_horizon,
		);
		let next = select_tactic(
			knowledge.is_empty(),
			management.tactic,
			targeting.is_some(),
			evasion.is_some(),
			management.scores(health_fraction, proximity),
			management.commitment,
		);
		if next != management.tactic {
			let from = management.tactic;
			management.generation = management.generation.wrapping_add(1).max(1);
			management.tactic = next;
			changed.write(ThreatTacticChanged {
				entity,
				from,
				to: next,
				generation: management.generation,
			});
		}
		apply_tactic(
			&mut commands,
			entity,
			knowledge,
			next,
			targeting.as_deref_mut(),
			evasion.as_deref_mut(),
		);
	}
}

fn apply_tactic(
	commands: &mut Commands,
	entity: Entity,
	knowledge: &ThreatKnowledge,
	tactic: ThreatTactic,
	targeting: Option<&mut CombatTargeting>,
	evasion: Option<&mut EvasionIntelligenceUser>,
) {
	match tactic {
		ThreatTactic::Combat => {
			commands.entity(entity).insert(CombatSelected).remove::<EvadeSelected>();
		}
		ThreatTactic::Evade => {
			commands.entity(entity).insert(EvadeSelected).remove::<CombatSelected>();
		}
		ThreatTactic::Ignore => {
			commands.entity(entity).remove::<(CombatSelected, EvadeSelected)>();
		}
	}
	if let Some(targeting) = targeting {
		grant_combat(targeting, knowledge, tactic == ThreatTactic::Combat);
	}
	if let Some(evasion) = evasion {
		grant_evade(evasion, knowledge, tactic == ThreatTactic::Evade);
	}
}

fn known_entities(knowledge: &ThreatKnowledge) -> HashSet<Entity> {
	knowledge.iter().filter_map(|known| known.entity).collect()
}

fn grant_combat(targeting: &mut CombatTargeting, knowledge: &ThreatKnowledge, granted: bool) {
	targeting.enabled = granted;
	if granted {
		sync_combat_membership(targeting, &known_entities(knowledge));
	} else {
		targeting.clear_source(TargetSource::ENEMYSHIP);
		targeting.clear_source(TargetSource::SPOTTING);
		targeting.ranked.clear();
		targeting.clear_engagement();
	}
}

fn grant_evade(evasion: &mut EvasionIntelligenceUser, knowledge: &ThreatKnowledge, granted: bool) {
	evasion.enabled = granted;
	if granted {
		sync_evasion_membership(evasion, &known_entities(knowledge));
	} else {
		evasion.clear_source(AssailantSource::ENEMYSHIP);
		evasion.clear_source(AssailantSource::SPOTTING);
		evasion.ranked.clear();
		evasion.signal = evasion_intelligence::EvasionSignal::idle();
	}
}

fn sync_combat_membership(targeting: &mut CombatTargeting, active: &HashSet<Entity>) {
	for subject in active {
		targeting.include(*subject, TargetSource::ENEMYSHIP);
	}
	let retired: Vec<_> = targeting
		.active
		.iter()
		.filter_map(|(subject, target)| {
			(target.has_source(TargetSource::ENEMYSHIP) && !active.contains(subject))
				.then_some(*subject)
		})
		.collect();
	for subject in retired {
		targeting.remove_source(subject, TargetSource::ENEMYSHIP);
	}
}

fn sync_evasion_membership(evasion: &mut EvasionIntelligenceUser, active: &HashSet<Entity>) {
	for subject in active {
		evasion.include(*subject, AssailantSource::ENEMYSHIP);
	}
	let retired: Vec<_> = evasion
		.active
		.iter()
		.filter_map(|(subject, assailant)| {
			(assailant.has_source(AssailantSource::ENEMYSHIP) && !active.contains(subject))
				.then_some(*subject)
		})
		.collect();
	for subject in retired {
		evasion.remove_source(subject, AssailantSource::ENEMYSHIP);
	}
}
