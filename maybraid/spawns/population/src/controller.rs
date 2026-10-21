//! Population demand → placement / actor / cancellation requests.

use bevy::prelude::*;
use maybraid_spawns_actors::{ActorRecipe, CancelActorSpawn, SpawnActor};
use maybraid_spawns_core::SpawnIdentity;
use maybraid_spawns_placement::{
	ActorClearance, CancelPlacement, PlacementReady, ResolvePlacement,
};
use maybraid_spawns_player::ArrivalDestination;

use crate::ownership::{PopulationOwner, PopulationOwnership};

/// Cell / encounter wants a member at a destination.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PopulationDemand {
	pub owner: PopulationOwner,
	pub destination: ArrivalDestination,
	pub fallback_at: Vec3,
	pub slot: u16,
}

impl PopulationDemand {
	pub const fn new(
		owner: PopulationOwner,
		destination: ArrivalDestination,
		fallback_at: Vec3,
		slot: u16,
	) -> Self {
		Self { owner, destination, fallback_at, slot }
	}
}

/// Encounter or cell retired. Pending work for that owner is cancelled.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PopulationRetired {
	pub owner: PopulationOwner,
}

impl PopulationRetired {
	pub const fn new(owner: PopulationOwner) -> Self {
		Self { owner }
	}

	pub const fn encounter(encounter: Entity) -> Self {
		Self { owner: PopulationOwner::new(encounter, 0) }
	}
}

/// Demand plus the identity stamped for placement / actor correlation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrackedDemand {
	pub identity: SpawnIdentity,
	pub demand: PopulationDemand,
}

/// Converts demand and retirement into spawn-domain requests.
pub struct PopulationController;

impl PopulationController {
	pub fn track(ownership: &mut PopulationOwnership, demand: PopulationDemand) -> TrackedDemand {
		TrackedDemand { identity: ownership.begin(demand.owner), demand }
	}

	pub fn placement(tracked: TrackedDemand) -> ResolvePlacement {
		ResolvePlacement::new(
			tracked.identity,
			tracked.demand.destination,
			ActorClearance::HUMANOID,
			tracked.demand.fallback_at,
		)
	}

	pub fn actor(ready: PlacementReady, recipe: ActorRecipe) -> SpawnActor {
		SpawnActor::new(ready.identity, ready.position, ready.policy.interior(), recipe)
	}

	pub fn cancel(identities: &[SpawnIdentity]) -> (Vec<CancelPlacement>, Vec<CancelActorSpawn>) {
		let placements = identities.iter().copied().map(CancelPlacement::new).collect();
		let actors = identities.iter().copied().map(CancelActorSpawn::new).collect();
		(placements, actors)
	}
}

pub fn begin_population_placements(
	mut ownership: ResMut<PopulationOwnership>,
	mut demand: MessageReader<PopulationDemand>,
	mut placements: MessageWriter<ResolvePlacement>,
) {
	for demand in demand.read().copied() {
		placements.write(PopulationController::placement(PopulationController::track(
			&mut ownership,
			demand,
		)));
	}
}

pub fn retire_population(
	mut ownership: ResMut<PopulationOwnership>,
	mut retired: MessageReader<PopulationRetired>,
	mut placements: MessageWriter<CancelPlacement>,
	mut actors: MessageWriter<CancelActorSpawn>,
) {
	for retired in retired.read().copied() {
		let identities = if retired.owner.cell == 0 {
			ownership.retire_encounter(retired.owner.encounter)
		} else {
			ownership.retire(retired.owner)
		};
		let (cancel_place, cancel_actor) = PopulationController::cancel(&identities);
		for cancel in cancel_place {
			placements.write(cancel);
		}
		for cancel in cancel_actor {
			actors.write(cancel);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_spawns_placement::{PlacementPolicy, PlacementPose, PlacementWork};
	use maybraid_spawns_player::ArrivalDestination;

	#[test]
	fn retirement_cancels_placement_and_actor_work() {
		let mut ownership = PopulationOwnership::default();
		let owner = PopulationOwner::new(Entity::from_bits(8), 3);
		let tracked = PopulationController::track(
			&mut ownership,
			PopulationDemand::new(owner, ArrivalDestination::Fallback, Vec3::Y, 0),
		);
		let mut work = PlacementWork::default();
		work.insert(maybraid_spawns_placement::PendingPlacement::new(
			tracked.identity,
			ArrivalDestination::Fallback,
			PlacementPose { position: Vec3::Y, policy: PlacementPolicy::OutdoorGround },
		));
		let retired = ownership.retire(owner);
		let (placements, actors) = PopulationController::cancel(&retired);
		assert_eq!(placements.len(), 1);
		assert_eq!(actors.len(), 1);
		assert!(work.cancel(tracked.identity).is_some());
	}
}
