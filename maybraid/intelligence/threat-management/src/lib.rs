//! Exclusive Ignore | Evade | Combat grant over retained threat knowledge.
//!
//! [`ThreatManagementIntelligence`] scores those tactics from remaining health
//! and nearest-known proximity. It does not classify who is a threat.

mod plugin;
mod scoring;
mod select;
mod tactic;
mod user;

pub use plugin::{ThreatManagementPlugin, ThreatManagementSystems};
pub use scoring::{
	meets_commitment, nearest_known_xz, proximity, score_tactics, select_tactic, TacticScores,
	ThreatManagementElement,
};
pub use select::select_threat_tactics;
pub use tactic::{CombatSelected, EvadeSelected, ThreatTactic, ThreatTacticChanged};
pub use user::ThreatManagementIntelligence;

#[cfg(test)]
mod tests;
