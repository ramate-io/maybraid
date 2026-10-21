//! Cancel pending construction and clean partial work.

use bevy::prelude::*;
use maybraid_spawns_core::{SpawnCancelReason, SpawnIdentity, SpawnOutcome};

/// Session-scoped in-flight actor construction. Keyed by request id.
#[derive(Resource, Default, Debug)]
pub struct ActorSpawnWork {
	pending: Vec<SpawnIdentity>,
}

impl ActorSpawnWork {
	pub fn insert(&mut self, identity: SpawnIdentity) {
		self.pending.retain(|pending| pending.request != identity.request);
		self.pending.push(identity);
	}

	pub fn contains(&self, identity: SpawnIdentity) -> bool {
		self.pending.iter().any(|pending| pending.matches(identity))
	}

	pub fn complete(&mut self, identity: SpawnIdentity) -> bool {
		let index = self.pending.iter().position(|pending| pending.matches(identity));
		if let Some(index) = index {
			self.pending.remove(index);
			true
		} else {
			false
		}
	}

	pub fn cancel(&mut self, identity: SpawnIdentity) -> Option<SpawnOutcome> {
		if !self.complete(identity) {
			return None;
		}
		Some(SpawnOutcome::cancelled(identity, SpawnCancelReason::Requested))
	}

	pub fn cancel_session(&mut self, session: maybraid_spawns_core::SpawnSessionId) {
		self.pending.retain(|pending| pending.owner.session != session);
	}

	pub fn is_empty(&self) -> bool {
		self.pending.is_empty()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_spawns_core::{SpawnOwner, SpawnRequestId, SpawnSessionId};

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	#[test]
	fn cancel_drops_later_completion() {
		let mut work = ActorSpawnWork::default();
		work.insert(identity());
		assert!(work.cancel(identity()).is_some());
		assert!(!work.complete(identity()));
		assert!(work.is_empty());
	}
}
