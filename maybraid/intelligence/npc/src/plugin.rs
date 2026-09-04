use bevy::prelude::*;
use idling_intelligence::{IdlingIntelligenceUser, IdlingSystems};
use meandering_intelligence::MeanderingIntelligenceUser;
use poi_intelligence::{PoiGoal, PoiSystems};
use tether_intelligence::{TetherIntelligenceUser, TetherSystems};
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
		Option<&'static mut IdlingIntelligenceUser>,
		Option<&'static mut TetherIntelligenceUser>,
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
				.before(TetherSystems::Write)
				.before(IdlingSystems::Write),
		)
		.add_systems(Update, mix_npc_brains.in_set(NpcIntelligenceSystems::Mix));
	}
}

/// Priority mixer: Combat/Evade preempt tether, meander, and idle; Ignore
/// restores them.
pub fn mix_npc_brains(mut commands: Commands, mut npcs: NpcMixers) {
	for (entity, npc, management, mut meandering, mut idling, mut tether, has_goal) in &mut npcs {
		let tactic = management.tactic;
		let acting = tactic != ThreatTactic::Ignore;
		if let Some(meandering) = meandering.as_deref_mut() {
			meandering.enabled = !acting;
		}
		if let Some(idling) = idling.as_deref_mut() {
			idling.enabled = !acting;
		}
		if acting && has_goal {
			commands.entity(entity).remove::<PoiGoal>();
		}
		if let Some(tether) = tether.as_deref_mut() {
			apply_tether(npc, tactic, tether);
		}
	}
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
