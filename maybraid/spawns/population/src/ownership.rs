//! Cell / encounter ownership. Retirement cancels that owner's pending work.

use bevy::prelude::*;
use maybraid_spawns_core::{
	SpawnCancelReason, SpawnIdentity, SpawnOutcome, SpawnOwner, SpawnRequestId, SpawnSessionId,
};

/// Who owns a population spawn. Cell and encounter stay here, not in a registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PopulationOwner {
	pub encounter: Entity,
	pub cell: u64,
}

impl PopulationOwner {
	pub const fn new(encounter: Entity, cell: u64) -> Self {
		Self { encounter, cell }
	}

	pub fn matches(self, other: Self) -> bool {
		self.encounter == other.encounter && self.cell == other.cell
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OwnedSpawn {
	identity: SpawnIdentity,
	owner: PopulationOwner,
}

/// Session-scoped ownership of population spawn identities.
#[derive(Resource, Debug)]
pub struct PopulationOwnership {
	session: SpawnSessionId,
	next_request: SpawnRequestId,
	owned: Vec<OwnedSpawn>,
}

impl Default for PopulationOwnership {
	fn default() -> Self {
		Self {
			session: SpawnSessionId::new(1),
			next_request: SpawnRequestId::new(1),
			owned: Vec::new(),
		}
	}
}

impl PopulationOwnership {
	pub fn owner(&self) -> SpawnOwner {
		SpawnOwner::new(self.session)
	}

	pub fn begin(&mut self, owner: PopulationOwner) -> SpawnIdentity {
		let identity = SpawnIdentity::new(self.next_request, self.owner());
		self.next_request = self.next_request.next();
		self.owned.retain(|owned| owned.identity.request != identity.request);
		self.owned.push(OwnedSpawn { identity, owner });
		identity
	}

	pub fn owner_of(&self, identity: SpawnIdentity) -> Option<PopulationOwner> {
		self.owned
			.iter()
			.find(|owned| owned.identity.matches(identity))
			.map(|owned| owned.owner)
	}

	pub fn retire(&mut self, owner: PopulationOwner) -> Vec<SpawnIdentity> {
		let mut retired = Vec::new();
		self.owned.retain(|owned| {
			if owned.owner.matches(owner) {
				retired.push(owned.identity);
				false
			} else {
				true
			}
		});
		retired
	}

	pub fn retire_encounter(&mut self, encounter: Entity) -> Vec<SpawnIdentity> {
		let mut retired = Vec::new();
		self.owned.retain(|owned| {
			if owned.owner.encounter == encounter {
				retired.push(owned.identity);
				false
			} else {
				true
			}
		});
		retired
	}

	pub fn outcomes(identities: &[SpawnIdentity]) -> Vec<SpawnOutcome> {
		identities
			.iter()
			.copied()
			.map(|identity| SpawnOutcome::cancelled(identity, SpawnCancelReason::OwnerRetired))
			.collect()
	}

	pub fn is_empty(&self) -> bool {
		self.owned.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn retiring_an_encounter_drops_its_identities() {
		let mut ownership = PopulationOwnership::default();
		let host = Entity::from_bits(3);
		let first = ownership.begin(PopulationOwner::new(host, 10));
		let _other = ownership.begin(PopulationOwner::new(Entity::from_bits(4), 11));
		let retired = ownership.retire_encounter(host);
		assert_eq!(retired, vec![first]);
		assert!(ownership.owner_of(first).is_none());
		assert_eq!(ownership.owned.len(), 1);
	}
}
