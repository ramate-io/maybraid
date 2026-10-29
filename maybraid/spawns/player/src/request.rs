//! Player-arrival requests. Destination selection stays here; placement and
//! actor construction are later interpreters.

use crate::identity::{SpawnIdentity, SpawnOwner};
use bevy::prelude::*;
use poi_intelligence::PoiId;
use world_player::RespawnOrigin;

/// Why this arrival started. The controller decides; interpreters do not
/// rediscover first-spawn vs death vs resume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArrivalReason {
	FirstSpawn { at: Vec3 },
	Death { at: Vec3 },
	Resume { at: Vec3 },
}

impl ArrivalReason {
	pub const fn origin_at(self) -> Vec3 {
		match self {
			Self::FirstSpawn { at } | Self::Death { at } | Self::Resume { at } => at,
		}
	}

	pub const fn first_life(self) -> bool {
		matches!(self, Self::FirstSpawn { .. })
	}

	pub const fn skips_picker(self) -> bool {
		matches!(self, Self::Resume { .. })
	}
}

/// Session start, player retirement, or resume asked for an arrival.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct BeginPlayerArrival {
	pub owner: SpawnOwner,
	pub reason: ArrivalReason,
	pub origin: RespawnOrigin,
}

impl BeginPlayerArrival {
	pub const fn new(owner: SpawnOwner, reason: ArrivalReason, origin: RespawnOrigin) -> Self {
		Self { owner, reason, origin }
	}
}

/// Where the picker (or a resume / in-place policy) placed this arrival.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArrivalDestination {
	Poi(PoiId),
	Fallback,
	Resume(Vec3),
	DeathInPlace(Vec3),
}

impl ArrivalDestination {
	pub const fn from_picker(poi: Option<PoiId>) -> Self {
		match poi {
			Some(id) => Self::Poi(id),
			None => Self::Fallback,
		}
	}

	pub const fn poi(self) -> Option<PoiId> {
		match self {
			Self::Poi(id) => Some(id),
			Self::Fallback | Self::Resume(_) | Self::DeathInPlace(_) => None,
		}
	}
}

/// Picker or policy selected a destination for one arrival.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct DestinationSelected {
	pub identity: SpawnIdentity,
	pub destination: ArrivalDestination,
}

impl DestinationSelected {
	pub const fn new(identity: SpawnIdentity, destination: ArrivalDestination) -> Self {
		Self { identity, destination }
	}

	pub fn matches(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}
}

/// Discovery map picker confirmed a POI, or `None` for the fallback ring.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlayerChoseRespawnPoi {
	pub identity: SpawnIdentity,
	pub poi: Option<PoiId>,
}

impl PlayerChoseRespawnPoi {
	pub const fn new(identity: SpawnIdentity, poi: Option<PoiId>) -> Self {
		Self { identity, poi }
	}

	pub fn matches(self, identity: SpawnIdentity) -> bool {
		self.identity.matches(identity)
	}

	pub fn destination(self) -> ArrivalDestination {
		ArrivalDestination::from_picker(self.poi)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::identity::{SpawnRequestId, SpawnSessionId};

	#[test]
	fn a_stale_destination_does_not_match_a_new_request() {
		let old =
			SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)));
		let next = SpawnIdentity::new(old.request.next(), old.owner);
		let selected = DestinationSelected::new(old, ArrivalDestination::Fallback);
		assert!(!selected.matches(next));
	}
}
