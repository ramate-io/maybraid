//! Persistent placement work. A request must not disappear because
//! generation outlives message retention.

use bevy::prelude::*;
use maybraid_spawns_player::ArrivalDestination;
use maybraid_spawns_player::{
	PlacementAttempt, SpawnCancelReason, SpawnFailureReason, SpawnIdentity, SpawnOutcome,
	SpawnSessionId,
};

use crate::request::{PlacementFailed, PlacementPolicy, PlacementReady, PlacementSupportHit};
use crate::support::{PlacementSupport, SupportReadiness};
use crate::terrain::{PlacementPose, PlacementSearch};
use characters::LocomotionCapsule;

/// One in-flight placement. Lives in a resource, not a one-frame message.
#[derive(Clone, Debug, PartialEq)]
pub struct PendingPlacement {
	pub identity: SpawnIdentity,
	pub attempt: PlacementAttempt,
	pub destination: ArrivalDestination,
	pub search: PlacementSearch,
	refresh_terrain: bool,
	pub accepted: Option<PlacementPose>,
	pub support: Option<PlacementSupportHit>,
	pub policy: PlacementPolicy,
	pub hull: LocomotionCapsule,
	pub readiness: SupportReadiness,
	pub failed: Option<SpawnFailureReason>,
}

impl PendingPlacement {
	pub fn new(
		identity: SpawnIdentity,
		attempt: PlacementAttempt,
		destination: ArrivalDestination,
		search: PlacementSearch,
	) -> Self {
		Self {
			identity,
			attempt,
			destination,
			policy: search.policy,
			hull: search.hull,
			search,
			refresh_terrain: false,
			accepted: None,
			support: None,
			readiness: SupportReadiness::default(),
			failed: None,
		}
	}

	/// Generated destination heights are refreshed until the shared collider
	/// probe accepts support; authored POI search bands remain fixed.
	pub fn with_published_terrain(mut self) -> Self {
		self.refresh_terrain = true;
		self
	}

	pub const fn refreshes_terrain(&self) -> bool {
		self.refresh_terrain
	}

	pub fn identity(&self) -> SpawnIdentity {
		self.identity
	}

	pub fn observe(&mut self, readiness: SupportReadiness) {
		self.readiness = readiness;
	}

	pub fn accept(&mut self, pose: PlacementPose, support: Option<PlacementSupportHit>) {
		self.accepted = Some(pose);
		self.support = support;
		self.readiness = SupportReadiness::presented();
	}

	pub fn fail(&mut self, reason: SpawnFailureReason) {
		self.failed = Some(reason);
	}

	pub fn ready(&self, support: PlacementSupport) -> Option<PlacementReady> {
		self.ready_with(support.readiness)
	}

	pub fn ready_with(&self, readiness: SupportReadiness) -> Option<PlacementReady> {
		if self.failed.is_some() {
			return None;
		}
		let pose = self.accepted?;
		if pose.policy.waits_for_support() && !readiness.ready() {
			return None;
		}
		Some(PlacementReady {
			identity: self.identity,
			attempt: self.attempt,
			position: pose.position,
			policy: pose.policy,
			support_ready: !pose.policy.waits_for_support() || readiness.ready(),
			support: self.support,
		})
	}

	pub fn tick(&self) -> Option<PlacementTick> {
		if let Some(reason) = self.failed {
			return Some(PlacementTick::Failed(PlacementFailed::new(
				self.identity,
				self.attempt,
				reason,
			)));
		}
		self.ready_with(self.readiness).map(PlacementTick::Ready)
	}

	pub fn fail_unresolved(&self) -> PlacementFailed {
		PlacementFailed::new(self.identity, self.attempt, SpawnFailureReason::DestinationUnresolved)
	}
}

/// Ready or failed for one pending request.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlacementTick {
	Ready(PlacementReady),
	Failed(PlacementFailed),
}

/// Session-scoped pending placements. Keyed by request id.
#[derive(Resource, Default, Debug)]
pub struct PlacementWork {
	pending: Vec<PendingPlacement>,
}

impl PlacementWork {
	pub fn insert(&mut self, placement: PendingPlacement) {
		self.pending.retain(|pending| {
			pending.identity.request != placement.identity.request
				|| pending.attempt != placement.attempt
		});
		self.pending.retain(|pending| !pending.identity.matches(placement.identity));
		self.pending.push(placement);
	}

	pub fn get(&self, identity: SpawnIdentity) -> Option<&PendingPlacement> {
		self.pending.iter().find(|pending| pending.identity.matches(identity))
	}

	pub fn get_mut(&mut self, identity: SpawnIdentity) -> Option<&mut PendingPlacement> {
		self.pending.iter_mut().find(|pending| pending.identity.matches(identity))
	}

	pub fn pending_mut(&mut self) -> &mut [PendingPlacement] {
		&mut self.pending
	}

	pub fn cancel(&mut self, identity: SpawnIdentity) -> Option<SpawnOutcome> {
		let index = self.pending.iter().position(|pending| pending.identity.matches(identity))?;
		let pending = self.pending.remove(index);
		Some(SpawnOutcome::cancelled(pending.identity, SpawnCancelReason::Requested))
	}

	pub fn cancel_session(&mut self, session: SpawnSessionId) {
		self.pending.retain(|pending| pending.identity.owner.session != session);
	}

	/// Drop work that no longer matches the live arrival and attempt.
	pub fn retain_identity(&mut self, identity: Option<SpawnIdentity>) {
		self.pending
			.retain(|pending| identity.is_some_and(|id| pending.identity.matches(id)));
	}

	pub fn retain_attempt(
		&mut self,
		identity: Option<SpawnIdentity>,
		attempt: Option<PlacementAttempt>,
	) {
		self.pending.retain(|pending| {
			identity.is_some_and(|id| pending.identity.matches(id))
				&& attempt.is_some_and(|attempt| pending.attempt == attempt)
		});
	}

	/// Apply one support snapshot to every request. Tests use this; production
	/// observes each column, then [`Self::poll_observed`].
	pub fn poll(&mut self, support: PlacementSupport) -> Vec<PlacementReady> {
		for pending in &mut self.pending {
			pending.observe(support.readiness);
		}
		self.poll_observed()
			.into_iter()
			.filter_map(|tick| match tick {
				PlacementTick::Ready(ready) => Some(ready),
				PlacementTick::Failed(_) => None,
			})
			.collect()
	}

	pub fn poll_each(
		&mut self,
		mut probe: impl FnMut(&PendingPlacement) -> SupportReadiness,
	) -> Vec<PlacementReady> {
		for pending in &mut self.pending {
			let readiness = probe(pending);
			pending.observe(readiness);
		}
		self.poll_observed()
			.into_iter()
			.filter_map(|tick| match tick {
				PlacementTick::Ready(ready) => Some(ready),
				PlacementTick::Failed(_) => None,
			})
			.collect()
	}

	pub fn poll_observed(&mut self) -> Vec<PlacementTick> {
		let mut ticks = Vec::new();
		self.pending.retain(|pending| match pending.tick() {
			Some(tick) => {
				ticks.push(tick);
				false
			}
			None => true,
		});
		ticks
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
	use maybraid_spawns_player::{SpawnOwner, SpawnRequestId, SpawnSessionId};

	fn identity_n(n: u64) -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(n), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	fn identity() -> SpawnIdentity {
		identity_n(1)
	}

	fn attempt() -> PlacementAttempt {
		PlacementAttempt::new(1)
	}

	fn outdoor_for(id: SpawnIdentity, at: Vec3) -> PendingPlacement {
		let mut pending = PendingPlacement::new(
			id,
			attempt(),
			ArrivalDestination::Fallback,
			PlacementSearch::outdoor_hint(at.xz(), at.y, LocomotionCapsule::HUMANOID),
		);
		pending.accept(
			PlacementPose::new(at, PlacementPolicy::OutdoorGround, LocomotionCapsule::HUMANOID),
			None,
		);
		pending
	}

	fn outdoor(at: Vec3) -> PendingPlacement {
		outdoor_for(identity(), at)
	}

	#[test]
	fn delayed_support_keeps_the_request() {
		let mut work = PlacementWork::default();
		let mut pending = outdoor(Vec3::Y);
		pending.accepted = None;
		work.insert(pending);
		assert!(work.poll(PlacementSupport::waiting()).is_empty());
		assert_eq!(work.len(), 1);
		work.get_mut(identity()).expect("pending").accept(
			PlacementPose::new(
				Vec3::Y,
				PlacementPolicy::OutdoorGround,
				LocomotionCapsule::HUMANOID,
			),
			None,
		);
		let ready = work.poll(PlacementSupport::ready());
		assert_eq!(ready.len(), 1);
		assert!(work.is_empty());
		assert!(ready[0].support_ready);
		assert!(ready[0].matches_attempt(identity(), attempt()));
	}

	#[test]
	fn poll_releases_only_the_covered_destination() {
		let first = identity_n(1);
		let second = identity_n(2);
		let mut work = PlacementWork::default();
		work.insert(outdoor_for(first, Vec3::new(0.0, 1.0, 0.0)));
		work.insert(outdoor_for(second, Vec3::new(100.0, 1.0, 0.0)));
		let ready =
			work.poll_each(|pending| SupportReadiness::new(true, pending.search.xz == Vec2::ZERO));
		assert_eq!(ready.len(), 1);
		assert!(ready[0].belongs_to(first));
		assert_eq!(work.len(), 1);
		assert!(work.get(second).is_some());
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
	fn an_interior_floor_waits_for_its_building_collider() {
		let pending = PendingPlacement::new(
			identity(),
			attempt(),
			ArrivalDestination::Poi(poi_intelligence::PoiId(1)),
			PlacementSearch::interior_at(Vec3::new(0.0, 12.0, 0.0), LocomotionCapsule::HUMANOID),
		);
		assert!(pending.ready(PlacementSupport::waiting()).is_none());
		assert!(pending.ready(PlacementSupport::ready()).is_none());
	}

	#[test]
	fn blocked_clearance_fails_instead_of_releasing() {
		let mut work = PlacementWork::default();
		let mut pending = outdoor(Vec3::Y);
		pending.fail(SpawnFailureReason::Clearance);
		work.insert(pending);
		let ticks = work.poll_observed();
		assert_eq!(ticks.len(), 1);
		match ticks[0] {
			PlacementTick::Failed(failed) => {
				assert_eq!(failed.reason, SpawnFailureReason::Clearance);
				assert!(failed.matches_attempt(identity(), attempt()));
			}
			PlacementTick::Ready(_) => panic!("blocked clearance must not become ready"),
		}
		assert!(work.is_empty());
	}

	#[test]
	fn replacing_an_attempt_drops_the_previous_request() {
		let mut work = PlacementWork::default();
		work.insert(outdoor(Vec3::Y));
		let mut next = outdoor(Vec3::Y);
		next.attempt = PlacementAttempt::new(2);
		work.insert(next);
		assert_eq!(work.len(), 1);
		assert_eq!(
			work.get(identity()).map(|pending| pending.attempt),
			Some(PlacementAttempt::new(2))
		);
	}
}
