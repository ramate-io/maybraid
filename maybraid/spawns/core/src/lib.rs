//! Shared spawn request identity and outcome vocabulary.
//!
//! Domain crates keep their own request types. This crate is optional for
//! simple consumers: it does not enumerate spawnable kinds.

pub mod outcome;
pub mod request;

pub use outcome::{SpawnCancelReason, SpawnFailureReason, SpawnOutcome};
pub use request::{SpawnIdentity, SpawnOwner, SpawnRequestId, SpawnSessionId};
