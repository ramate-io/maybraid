//! Semantic victim classes for Bikanwarda combat weights.

use threat_intelligence::{Affiliations, ThreatGroupId};

/// Mob-scene group ids ([`mob_scenes::brain`](../../../world/layers/mobs/scenes/src/brain.rs)).
pub const HERD_GROUP: ThreatGroupId = ThreatGroupId::group(30);
pub const PACK_GROUP: ThreatGroupId = ThreatGroupId::group(31);
pub const RAIDER_GROUP: ThreatGroupId = ThreatGroupId::group(32);

/// Victim bucket for charge weights.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BikanwardaVictimClass {
	Animal,
	Raider,
	Neutral,
}

impl BikanwardaVictimClass {
	pub fn from_affiliations(affiliations: &Affiliations) -> Self {
		if memberships_contain(affiliations, RAIDER_GROUP) {
			return Self::Raider;
		}
		if memberships_contain(affiliations, HERD_GROUP)
			|| memberships_contain(affiliations, PACK_GROUP)
		{
			return Self::Animal;
		}
		Self::Neutral
	}
}

fn memberships_contain(affiliations: &Affiliations, group: ThreatGroupId) -> bool {
	affiliations.memberships.contains_key(&group)
}
