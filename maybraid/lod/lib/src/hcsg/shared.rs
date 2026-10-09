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
//! - [`bounds`]: [`HcsgBounds`], where a system wants values.
//! - [`generation`]: [`generation<B, T>`](generation::generation) keeps `T`
//!   warm within `B`, fire and forget.
//! - [`presentation`]: [`presentation<B, T>`](presentation::presentation)
//!   keeps one [`HcsgNode<T>`] host per published value within `B`.
//! - [`node`]: [`HcsgNode<T>`], forwarding the LOD scene traits to `T`.
//!
//! This sits alongside the frame-synchronous [`super::HcsgStorage`] until the
//! layers move over.

pub mod bounds;
pub mod context;
pub mod demand;
pub mod generation;
pub mod node;
pub mod presentation;
mod runtime;
pub mod storage;
pub mod worker;

#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod tests;

pub use bounds::HcsgBounds;
pub use context::{GenerationContext, GenerationScheme};
pub use demand::{HcsgDemand, Published, SubscriptionId};
pub use generation::GenerationPlugin;
pub use node::HcsgNode;
pub use presentation::{PresentationPlugin, RetiredHost};
pub use runtime::HcsgSystems;
pub use storage::{Busy, HcsgStorage, HcsgValue};
pub use worker::HcsgWorker;
