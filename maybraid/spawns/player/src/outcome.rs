//! Cancellation and failure reasons for player-arrival outcomes.
//!
//! Placement and later interpreters can reuse these types from this crate
//! until real duplication justifies a shared vocabulary crate.

use crate::identity::SpawnIdentity;

/// Why pending spawn work stopped without realizing an entity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnCancelReason {
	/// The play session that issued the request ended.
	SessionEnded,
	/// A newer request replaced this one for the same owner.
	Superseded,
	/// An explicit cancel from the owning controller.
	Requested,
	/// The actor or cell that owned the request retired.
	OwnerRetired,
}

/// Why realization failed. Controllers may retry or fall back.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnFailureReason {
	/// Destination could not be resolved to a usable pose.
	Placement,
	/// Actor / body construction failed.
	Construction,
	/// Required collision or presentation support never became ready.
	SupportUnavailable,
	/// The chosen destination is gone or no longer valid.
	DestinationUnresolved,
	/// The capsule does not fit at the resolved pose.
	Clearance,
}

/// Outcome envelope for player-arrival work. Other domains embed their own
/// identity; they do not become a universal spawn enum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnOutcome {
	Cancelled { identity: SpawnIdentity, reason: SpawnCancelReason },
	Failed { identity: SpawnIdentity, reason: SpawnFailureReason },
}

impl SpawnOutcome {
	pub const fn cancelled(identity: SpawnIdentity, reason: SpawnCancelReason) -> Self {
		Self::Cancelled { identity, reason }
	}

	pub const fn failed(identity: SpawnIdentity, reason: SpawnFailureReason) -> Self {
		Self::Failed { identity, reason }
	}

	pub const fn identity(self) -> SpawnIdentity {
		match self {
			Self::Cancelled { identity, .. } | Self::Failed { identity, .. } => identity,
		}
	}

	pub const fn retryable(self) -> bool {
		matches!(
			self,
			Self::Failed {
				reason: SpawnFailureReason::Placement
					| SpawnFailureReason::Construction
					| SpawnFailureReason::SupportUnavailable
					| SpawnFailureReason::DestinationUnresolved
					| SpawnFailureReason::Clearance,
				..
			}
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::identity::{SpawnOwner, SpawnRequestId, SpawnSessionId};

	#[test]
	fn construction_failure_is_retryable_and_session_cancel_is_not() {
		let identity =
			SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)));
		assert!(SpawnOutcome::failed(identity, SpawnFailureReason::Construction).retryable());
		assert!(SpawnOutcome::failed(identity, SpawnFailureReason::Clearance).retryable());
		assert!(!SpawnOutcome::cancelled(identity, SpawnCancelReason::SessionEnded).retryable());
	}
}
