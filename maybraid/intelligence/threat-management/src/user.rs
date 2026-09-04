use bevy::prelude::*;

use crate::{ThreatManagementElement, ThreatTactic};

/// Installed exclusive Ignore | Evade | Combat policy.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ThreatManagementIntelligence {
	pub ignore: ThreatManagementElement,
	pub evade: ThreatManagementElement,
	pub combat: ThreatManagementElement,
	/// Required `score_new / score_old` to leave Combat or Evade while threats remain.
	pub commitment: (f32, f32),
	pub proximity_horizon: f32,
	pub selection_interval: f32,
	pub tactic: ThreatTactic,
	pub generation: u64,
	pub(crate) next_select_at: f32,
}

impl Default for ThreatManagementIntelligence {
	fn default() -> Self {
		Self {
			ignore: ThreatManagementElement::ZERO,
			evade: ThreatManagementElement::ZERO,
			combat: ThreatManagementElement::ZERO,
			commitment: (1.0, 1.0),
			proximity_horizon: 80.0,
			selection_interval: 0.25,
			tactic: ThreatTactic::Ignore,
			generation: 0,
			next_select_at: 0.0,
		}
	}
}

impl ThreatManagementIntelligence {
	/// FFA combatant: always Combat while any threat remains.
	pub fn ffa() -> Self {
		Self {
			combat: ThreatManagementElement::new(1.0, 1.0),
			commitment: (1.0, 0.0),
			..Self::default()
		}
	}

	/// AFFA civilian: always Evade while any threat remains.
	pub fn civilian() -> Self {
		Self {
			evade: ThreatManagementElement::new(1.0, 1.0),
			commitment: (1.0, 0.0),
			..Self::default()
		}
	}

	pub fn element(self, tactic: ThreatTactic) -> ThreatManagementElement {
		match tactic {
			ThreatTactic::Ignore => self.ignore,
			ThreatTactic::Evade => self.evade,
			ThreatTactic::Combat => self.combat,
		}
	}

	pub fn scores(self, health: f32, proximity: f32) -> crate::TacticScores {
		crate::score_tactics(self.ignore, self.evade, self.combat, health, proximity)
	}
}
