//! Persistent placement work. A request must not disappear because
//! generation outlives message retention.

use bevy::prelude::*;
use maybraid_spawns_core::{SpawnCancelReason, SpawnFailureReason, SpawnIdentity, SpawnOutcome};
use maybraid_spawns_player::ArrivalDestination;

use crate::request::{PlacementFailed, PlacementPolicy, PlacementReady};
use crate::support::PlacementSupport;
use crate::terrain::PlacementPose;

/// One in-flight placement. Lives in a resource, not a one-frame message.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingPlacement {
	pub identity: SpawnIdentity,
	pub destination: ArrivalDestination,
	pub pose: Option<PlacementPose>,
	pub policy: PlacementPolicy,
}

impl PendingPlacement {
	pub fn new(
		identity: SpawnIdentity,
		destination: ArrivalDestination,
		pose: PlacementPose,
	) -> Self {
		Self { identity, destination, pose: Some(pose), policy: pose.policy }
	}

	pub fn identity(&self) -> SpawnIdentity {
		self.identity
	}

	pub fn ready(&self, support: PlacementSupport) -> Option<PlacementReady> {
		let pose = self.pose?;
		if pose.policy.waits_for_support() && !support.is_ready() {
			return None;
		}
		Some(PlacementReady::new(
			self.identity,
			pose.position,
			pose.policy,
			!pose.policy.waits_for_support() || support.is_ready(),
		))
	}

	pub fn fail_unresolved(&self) -> PlacementFailed {
		PlacementFailed::new(self.identity, SpawnFailureReason::DestinationUnresolved)
	}
}

/// Session-scoped pending placements. Keyed by request id.
#[derive(Resource, Default, Debug)]
pub struct PlacementWork {
	pending: Vec<PendingPlacement>,
}

impl PlacementWork {
	pub fn insert(&mut self, placement: PendingPlacement) {
		self.pending
			.retain(|pending| pending.identity.request != placement.identity.request);
		self.pending.push(placement);
	}

	pub fn get(&self, identity: SpawnIdentity) -> Option<&PendingPlacement> {
		self.pending.iter().find(|pending| pending.identity.matches(identity))
	}

	pub fn cancel(&mut self, identity: SpawnIdentity) -> Option<SpawnOutcome> {
		let index = self.pending.iter().position(|pending| pending.identity.matches(identity))?;
		let pending = self.pending.remove(index);
		Some(SpawnOutcome::cancelled(pending.identity, SpawnCancelReason::Requested))
	}

	pub fn cancel_session(&mut self, session: maybraid_spawns_core::SpawnSessionId) {
		self.pending.retain(|pending| pending.identity.owner.session != session);
	}

	pub fn poll(&mut self, support: PlacementSupport) -> Vec<PlacementReady> {
		let mut ready = Vec::new();
		self.pending.retain(|pending| match pending.ready(support) {
			Some(outcome) => {
				ready.push(outcome);
				false
			}
			None => true,
		});
		ready
	}

	pub fn is_empty(&self) -> bool {
		self.pending.is_empty()
	}

	pub fn len(&self) -> usize {
		self.pending.len()
	}

	pub fn pending(&self) -> &[PendingPlacement] {
		&self.pending
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use maybraid_spawns_core::{SpawnOwner, SpawnRequestId, SpawnSessionId};
	use maybraid_spawns_player::ArrivalDestination;

	use crate::support::PlacementSupport;

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	fn outdoor(at: Vec3) -> PendingPlacement {
		PendingPlacement::new(
			identity(),
			ArrivalDestination::Fallback,
			PlacementPose { position: at, policy: PlacementPolicy::OutdoorGround },
		)
	}

	#[test]
	fn delayed_support_keeps_the_request() {
		let mut work = PlacementWork::default();
		work.insert(outdoor(Vec3::Y));
		assert!(work.poll(PlacementSupport::waiting()).is_empty());
		assert_eq!(work.len(), 1);
		let ready = work.poll(PlacementSupport::ready());
		assert_eq!(ready.len(), 1);
		assert!(work.is_empty());
		assert!(ready[0].support_ready);
	}

	#[test]
	fn cancel_during_loading_drops_a_later_ready() {
		let mut work = PlacementWork::default();
		work.insert(outdoor(Vec3::Y));
		assert!(work.cancel(identity()).is_some());
		assert!(work.poll(PlacementSupport::ready()).is_empty());
	}

	#[test]
	fn a_later_session_cancel_drops_stale_work() {
		let mut work = PlacementWork::default();
		work.insert(outdoor(Vec3::Y));
		work.cancel_session(SpawnSessionId::new(1));
		assert!(work.is_empty());
		assert!(work.poll(PlacementSupport::ready()).is_empty());
	}

	#[test]
	fn an_interior_floor_does_not_wait_for_terrain_support() {
		let pending = PendingPlacement::new(
			identity(),
			ArrivalDestination::Poi(poi_intelligence::PoiId(1)),
			PlacementPose {
				position: Vec3::new(0.0, 12.0, 0.0),
				policy: PlacementPolicy::InteriorFloor,
			},
		);
		let ready = pending.ready(PlacementSupport::waiting());
		assert!(ready
			.is_some_and(|ready| ready.position.y == 12.0 && !ready.policy.waits_for_support()));
	}
}
