//! Pack-level brain: roster, shared tether, affiliations, POI interests, travel.
//!
//! This crate does not implement LodScene and does not pick mob kinds. High
//! plants carry an Entity-free wish ([`MobSlot`], optional [`MobId`]); bind
//! writes the live [`MemberOf`] pointer. See [ROSTER.md](ROSTER.md).
//! Journeying hosts plan corridors with [`routing_intelligence`] and slide along
//! those hops; they are not movement-intelligence users.

mod bind;
mod host;
mod lifecycle;
mod lock;
mod member;
mod plugin;
mod roster;
mod travel;

pub use host::{
	install_mob, install_mob_journeying, install_mob_routing, spawn_mob, Mob, MobId, MobIdAlloc,
	MobInstall,
};
pub use lock::MobTetherLock;
pub use member::{MemberOf, MobMemberBody, MobSlot};
pub use plugin::{MobIntelligencePlugin, MobSystems};
pub use roster::{
	MobAffiliations, MobInterests, MobMemberNeeded, MobRespawn, MobRespawnAt, MobRoster,
	RosterMember,
};
pub use travel::MobTravel;

#[cfg(test)]
mod tests;
