use bevy::prelude::*;
use meandering_intelligence::MeanderingIntelligenceUser;
use poi_intelligence::{PoiGoal, PoiSystems};
use tether_intelligence::{TetherIntelligenceUser, TetherMemory, TetherSystems};
use threat_management_intelligence::{
	ThreatManagementIntelligence, ThreatManagementSystems, ThreatTactic,
};

use crate::NpcIntelligence;

type NpcMixers<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static NpcIntelligence,
		&'static ThreatManagementIntelligence,
		Option<&'static mut MeanderingIntelligenceUser>,
		Option<&'static mut TetherIntelligenceUser>,
		Option<&'static TetherMemory>,
		Has<PoiGoal>,
	),
>;

/// Exclusive grants after threat-management selects a tactic.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NpcIntelligenceSystems {
	Mix,
}

pub struct NpcIntelligencePlugin;

impl Plugin for NpcIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			NpcIntelligenceSystems::Mix
				.after(ThreatManagementSystems::Select)
				.before(PoiSystems::Select)
				.before(TetherSystems::Write),
		)
		.add_systems(Update, mix_npc_brains.in_set(NpcIntelligenceSystems::Mix));
	}
}

/// Priority mixer: Combat/Evade preempt tether and meander. Ignore restores
/// tether; meander only while that tether is satisfied (or absent).
pub fn mix_npc_brains(mut commands: Commands, mut npcs: NpcMixers) {
	for (entity, npc, management, mut meandering, mut tether, memory, has_goal) in &mut npcs {
		let tactic = management.tactic;
		let acting = tactic != ThreatTactic::Ignore;
		if let Some(tether) = tether.as_deref_mut() {
			apply_tether(npc, tactic, tether);
		}
		let pulling = tether_is_pulling(tether.as_deref(), memory);
		if let Some(meandering) = meandering.as_deref_mut() {
			meandering.enabled = !acting && !pulling;
		}
		if (acting || pulling) && has_goal {
			commands.entity(entity).remove::<PoiGoal>();
		}
	}
}

fn tether_is_pulling(
	tether: Option<&TetherIntelligenceUser>,
	memory: Option<&TetherMemory>,
) -> bool {
	tether.is_some_and(|tether| tether.enabled) && memory.is_some_and(|memory| !memory.satisfied)
}

fn apply_tether(npc: &NpcIntelligence, tactic: ThreatTactic, tether: &mut TetherIntelligenceUser) {
	match tactic {
		ThreatTactic::Ignore => {
			tether.enabled = true;
			if let Some(objective) = npc.idle_tether {
				tether.objective = objective;
			}
		}
		ThreatTactic::Combat => {
			if let Some(objective) = npc.engaged_tether.or(npc.idle_tether) {
				tether.objective = objective;
			}
			tether.enabled = npc.keep_tether_in_combat;
		}
		ThreatTactic::Evade => {
			tether.enabled = false;
		}
	}
}
