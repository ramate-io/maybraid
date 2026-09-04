//! Exclusive NPC mixer over threat, tether, and meander.
//!
//! Personalities are constructors: they stamp coefficients and which actuators
//! exist. They do not re-score tactics. [`ThreatManagementIntelligence`] remains
//! the Ignore | Evade | Combat grant.

mod personality;
mod plugin;
mod user;

pub use personality::{
	CombatPersonality, EvadePersonality, NpcBody, NpcInstall, Personality, PersonalitySpec,
	PersonalityTether,
};
pub use plugin::{mix_npc_brains, NpcIntelligencePlugin, NpcIntelligenceSystems};
pub use user::NpcIntelligence;

#[cfg(test)]
mod tests;
