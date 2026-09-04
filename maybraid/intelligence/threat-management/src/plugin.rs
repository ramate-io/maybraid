use bevy::prelude::*;
use combat_targeting::CombatTargetingSystems;
use evasion_intelligence::EvasionSystems;
use spotting_intelligence::SpottingSystems;
use threat_intelligence::ThreatSystems;

use crate::{select_threat_tactics, ThreatTacticChanged};

/// Low-cadence tactic selection after threat discovery.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ThreatManagementSystems {
	Select,
}

pub struct ThreatManagementPlugin;

impl Plugin for ThreatManagementPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<ThreatTacticChanged>()
			.configure_sets(
				Update,
				ThreatManagementSystems::Select
					.after(ThreatSystems::Discover)
					.before(ThreatSystems::Export)
					.before(SpottingSystems::Observe)
					.before(CombatTargetingSystems::Rank)
					.before(EvasionSystems::Ingest),
			)
			.add_systems(Update, select_threat_tactics.in_set(ThreatManagementSystems::Select));
	}
}
