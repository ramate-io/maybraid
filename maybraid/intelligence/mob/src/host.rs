//! Always-on mob host. Kind recipes stay in the caller.

use bevy::prelude::*;
use journeying_intelligence::JourneyingIntelligenceUser;
use poi_intelligence::{PoiIntelligenceUser, PoiInterests, PoiKnowledge, PoiVisitState};
use tether_intelligence::Tether;

use crate::roster::{MobAffiliations, MobInterests, MobRespawn, MobRoster, RosterMember};
use crate::travel::MobTravel;

/// Stable pack identity. Safe to stamp on High BSN; it is not an [`Entity`].
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MobId(pub u64);

/// Allocates [`MobId`]s for authored or procedural hosts.
#[derive(Resource, Debug, Default)]
pub struct MobIdAlloc(u64);

impl MobIdAlloc {
	pub fn allocate(&mut self) -> MobId {
		self.0 = self.0.saturating_add(1);
		MobId(self.0)
	}
}

/// Grove-level pack brain. Lives on the host for every LOD band.
#[derive(Component, Clone, Copy, Debug)]
pub struct Mob {
	pub leash: f32,
}

impl Mob {
	pub fn new(leash: f32) -> Self {
		Self { leash: leash.max(0.0) }
	}
}

/// What [`spawn_mob`] stamps on a new host.
#[derive(Clone, Debug)]
pub struct MobInstall {
	pub id: MobId,
	pub leash: f32,
	pub members: Vec<RosterMember>,
	pub interests: PoiInterests,
	pub affiliations: MobAffiliations,
	pub respawn: MobRespawn,
	pub travel: Option<MobTravel>,
	pub journey: bool,
}

impl MobInstall {
	pub fn new(id: MobId, leash: f32, members: Vec<RosterMember>) -> Self {
		Self {
			id,
			leash,
			members,
			interests: PoiInterests::default(),
			affiliations: MobAffiliations::default(),
			respawn: MobRespawn::default(),
			travel: None,
			journey: false,
		}
	}

	pub fn with_interests(mut self, interests: PoiInterests) -> Self {
		self.interests = interests;
		self
	}

	pub fn with_affiliations(mut self, affiliations: MobAffiliations) -> Self {
		self.affiliations = affiliations;
		self
	}

	pub fn with_respawn(mut self, respawn: MobRespawn) -> Self {
		self.respawn = respawn;
		self
	}

	pub fn with_travel(mut self, travel: MobTravel) -> Self {
		self.travel = Some(travel);
		self
	}

	pub fn with_journey(mut self, journey: bool) -> Self {
		self.journey = journey;
		self
	}
}

/// Spawn the always-on host: roster, tether marker, pack tables, optional travel.
pub fn spawn_mob(commands: &mut Commands, transform: Transform, install: MobInstall) -> Entity {
	let interests = install.interests.clone();
	let host = commands
		.spawn((
			Mob::new(install.leash),
			install.id,
			MobRoster::new(install.members),
			install.affiliations,
			install.respawn,
			MobInterests(interests.clone()),
			Tether,
			transform,
			Visibility::default(),
		))
		.id();
	if let Some(travel) = install.travel {
		commands.entity(host).insert(travel);
	}
	if install.journey {
		install_mob_journeying(commands, host, interests, install.id.0);
	}
	host
}

/// Distant-tile travel for **moving the tether**. The host must not meander.
pub fn install_mob_journeying(
	commands: &mut Commands,
	host: Entity,
	interests: PoiInterests,
	seed: u64,
) {
	commands.entity(host).insert((
		JourneyingIntelligenceUser::new(seed),
		PoiIntelligenceUser::new(interests),
		PoiKnowledge::default(),
		PoiVisitState::default(),
	));
}
