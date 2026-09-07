//! Roster slots and pack-level affiliation / respawn policy.

use bevy::prelude::*;
use damage::Health;
use firearm_intelligence::FirearmEngagement;
use npc_intelligence::{NpcBody, NpcInstall, Personality};
use poi_intelligence::{
	NearbyChoice, NearbyFallback, NearbyQuery, PoiId, PoiInterests, DEFAULT_NEARBY_RADIUS,
};

/// How many recent replace destinations a slot remembers for [`choose_in`].
pub const RECENT_POI_LIMIT: usize = 3;
/// Extra delay between slots so a wipe does not re-roll every member in one frame.
pub const RESPAWN_SLOT_STAGGER_SECS: f32 = 0.4;
use threat_intelligence::{Affiliations, ThreatId};
use threat_management_intelligence::ThreatManagementIntelligence;

use crate::MobId;

/// Pack membership / antagonism copied onto members at bind.
///
/// Do not put individual [`ThreatId`] groups here. [`Self::for_member`] adds
/// the member's self id.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct MobAffiliations(pub Affiliations);

/// Pack POI table copied onto members at bind.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct MobInterests(pub PoiInterests);

impl MobAffiliations {
	pub fn new(affiliations: Affiliations) -> Self {
		Self(affiliations)
	}

	pub fn for_member(&self, id: ThreatId) -> Affiliations {
		let mut affiliations = Affiliations::with_self(id);
		for (group, strength) in &self.0.memberships {
			affiliations.join(*group, *strength);
		}
		for (group, strength) in &self.0.known_antagonists {
			affiliations.antagonize(*group, *strength);
		}
		for (group, strength) in &self.0.known_allies {
			affiliations.mitigate(*group, *strength);
		}
		affiliations
	}
}

/// Where a replacement plant is stamped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MobRespawnAt {
	/// Host XZ, last recorded feet height.
	Host,
	/// A weighted nearby point of interest, with varied host-relative fallback.
	#[default]
	Poi,
	LastPose,
}

/// When a member dies, the roster can ask the app to spawn a new plant.
///
/// Missing this component means [`Self::never`]. High cull must not use this
/// path: it clears the live pointer and waits for fulfill.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct MobRespawn {
	pub delay_secs: f32,
	/// Replacements after the original plant. `None` is unlimited. `Some(0)` never.
	pub max_replacements: Option<u32>,
	pub at: MobRespawnAt,
	/// Readable corpse before despawn. Zero still drains on the next `Last`.
	pub corpse_secs: f32,
	/// Nearby POI scan used when [`Self::at`] is [`MobRespawnAt::Poi`].
	pub poi_radius: f32,
	/// Drop POIs closer than this on XZ. Zero keeps pack-local landmarks.
	pub min_radius: f32,
	/// Host-relative ring when no nearby POI is available.
	pub fallback: NearbyFallback,
	/// Extra seconds per roster slot added to [`Self::delay_secs`].
	pub slot_stagger_secs: f32,
}

impl Default for MobRespawn {
	fn default() -> Self {
		Self {
			delay_secs: 8.0,
			max_replacements: None,
			at: MobRespawnAt::Poi,
			corpse_secs: 4.0,
			poi_radius: DEFAULT_NEARBY_RADIUS,
			min_radius: 0.0,
			fallback: NearbyFallback::new(4.0, 12.0),
			slot_stagger_secs: RESPAWN_SLOT_STAGGER_SECS,
		}
	}
}

impl MobRespawn {
	pub fn never() -> Self {
		Self {
			delay_secs: 0.0,
			max_replacements: Some(0),
			at: MobRespawnAt::Host,
			corpse_secs: 4.0,
			..Self::default()
		}
	}

	pub fn nearby_query(self) -> NearbyQuery {
		NearbyQuery {
			radius: self.poi_radius,
			min_radius: self.min_radius,
			choice: NearbyChoice::Weighted,
		}
	}

	pub fn allows(self, replacements_used: u32) -> bool {
		match self.max_replacements {
			Some(0) => false,
			Some(max) => replacements_used < max,
			None => true,
		}
	}
}

/// Body-spawn request after a roster slot is empty long enough.
///
/// This crate does not spawn character controllers. The app inserts [`crate::MobSlot`]
/// (and optional [`crate::MobId`]) on the new body; bind fills the live pointer.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct MobMemberNeeded {
	pub mob: Entity,
	pub id: MobId,
	pub slot: u16,
	pub pose: Vec3,
}

/// One High plant spec plus the last known live snapshot.
#[derive(Clone, Debug)]
pub struct RosterMember {
	pub personality: Personality,
	pub armed: bool,
	pub keep_tether_in_combat: Option<bool>,
	pub discovery_radius: Option<f32>,
	pub spotting_range: Option<f32>,
	pub engagement: Option<FirearmEngagement>,
	pub threat_override: Option<ThreatManagementIntelligence>,
	pub interests: PoiInterests,
	pub pose: Vec3,
	pub health: Health,
	pub entity: Option<Entity>,
	pub replacements_used: u32,
	pub respawn_at: Option<f32>,
	pub spawn_requested: bool,
	pub last_respawn_poi: Option<PoiId>,
	pub recent_respawn_pois: Vec<PoiId>,
}

impl RosterMember {
	pub fn new(personality: Personality, pose: Vec3) -> Self {
		Self {
			personality,
			armed: true,
			keep_tether_in_combat: None,
			discovery_radius: None,
			spotting_range: None,
			engagement: None,
			threat_override: None,
			interests: PoiInterests::default(),
			pose,
			health: Health::default(),
			entity: None,
			replacements_used: 0,
			respawn_at: None,
			spawn_requested: false,
			last_respawn_poi: None,
			recent_respawn_pois: Vec::new(),
		}
	}

	pub fn remember_respawn_poi(&mut self, poi: Option<PoiId>) {
		self.last_respawn_poi = poi;
		let Some(id) = poi else {
			return;
		};
		self.recent_respawn_pois.retain(|known| *known != id);
		self.recent_respawn_pois.push(id);
		if self.recent_respawn_pois.len() > RECENT_POI_LIMIT {
			self.recent_respawn_pois.remove(0);
		}
	}

	pub fn excluded_pois(&self) -> &[PoiId] {
		&self.recent_respawn_pois
	}

	pub fn with_armed(mut self, armed: bool) -> Self {
		self.armed = armed;
		self
	}

	pub fn with_keep_tether_in_combat(mut self, keep: Option<bool>) -> Self {
		self.keep_tether_in_combat = keep;
		self
	}

	pub fn with_discovery_radius(mut self, radius: Option<f32>) -> Self {
		self.discovery_radius = radius;
		self
	}

	pub fn with_engagement(mut self, engagement: Option<FirearmEngagement>) -> Self {
		self.engagement = engagement;
		self
	}

	pub fn with_threat_override(
		mut self,
		threat_override: Option<ThreatManagementIntelligence>,
	) -> Self {
		self.threat_override = threat_override;
		self
	}

	pub fn with_interests(mut self, interests: PoiInterests) -> Self {
		self.interests = interests;
		self
	}

	pub fn npc_install(&self, host: Entity, at: Vec3, body: NpcBody) -> NpcInstall {
		NpcInstall {
			at,
			body,
			health: self.health,
			tether: Some(host),
			poi_interests: self.interests.clone(),
			engagement: self.engagement.clone(),
			threat_override: self.threat_override,
			discovery_radius: self.discovery_radius,
			spotting_range: self.spotting_range,
			armed: self.armed,
			keep_tether_in_combat: self.keep_tether_in_combat,
			selection_salt: 0,
		}
	}
}

/// Source of truth for Occupy / respawn while High plants are culled.
#[derive(Component, Clone, Debug, Default)]
pub struct MobRoster {
	members: Vec<RosterMember>,
}

impl MobRoster {
	pub fn new(members: Vec<RosterMember>) -> Self {
		Self { members }
	}

	pub fn len(&self) -> usize {
		self.members.len()
	}

	pub fn is_empty(&self) -> bool {
		self.members.is_empty()
	}

	pub fn get(&self, slot: u16) -> Option<&RosterMember> {
		self.members.get(usize::from(slot))
	}

	pub fn get_mut(&mut self, slot: u16) -> Option<&mut RosterMember> {
		self.members.get_mut(usize::from(slot))
	}

	pub fn iter(&self) -> impl Iterator<Item = (u16, &RosterMember)> {
		self.members.iter().enumerate().map(|(index, member)| (index as u16, member))
	}

	pub fn iter_mut(&mut self) -> impl Iterator<Item = (u16, &mut RosterMember)> {
		self.members
			.iter_mut()
			.enumerate()
			.map(|(index, member)| (index as u16, member))
	}

	/// Live or just-requested sibling poses, skipping `except`.
	pub fn occupied_poses(&self, except: Option<u16>) -> Vec<Vec3> {
		self.iter()
			.filter(|(slot, member)| {
				except.is_none_or(|skip| *slot != skip)
					&& (member.entity.is_some() || member.spawn_requested)
			})
			.map(|(_, member)| member.pose)
			.collect()
	}
}
