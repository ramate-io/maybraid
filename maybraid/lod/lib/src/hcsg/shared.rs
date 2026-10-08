//! Thread-shared HCSG runtime: generation runs on a worker, the frame only
//! subscribes bounds and reads published values. The design is
//! `hcsg/README.md`, next to this module.
//!
//! - [`storage`]: [`HcsgStorage`], a cloneable handle over one locked
//!   [`NodeStore`](super::NodeStore) of `Arc<T>` per type. Locks cover lookup
//!   and publication only.
//! - [`context`]: [`GenerationContext`] hands schemes owned `Arc` dependencies
//!   and generates what is missing.
//! - [`demand`]: [`HcsgDemand`], one subscription per bounds source, and the
//!   epoch that sessions advance.
//! - [`worker`]: [`HcsgWorker`], the thread that fills subscriptions.
//!
//! This sits alongside the frame-synchronous [`super::HcsgStorage`] until the
//! layers move over.

pub mod context;
pub mod demand;
pub mod storage;
pub mod worker;

#[cfg(test)]
mod tests;

pub use context::{GenerationContext, GenerationScheme};
pub use demand::{HcsgDemand, SubscriptionId};
pub use storage::{Busy, HcsgStorage, HcsgValue};
pub use worker::HcsgWorker;
