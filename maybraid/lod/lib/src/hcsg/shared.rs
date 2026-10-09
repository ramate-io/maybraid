//! Thread-shared HCSG runtime: generation runs on a worker, the frame only
//! subscribes bounds and reads published values. The design is
//! `hcsg/README.md`, next to this module.
//!
//! - [`storage`]: [`HcsgStorage`], a cloneable handle over one locked
//!   [`NodeStore`](node_store::NodeStore) of `Arc<T>` per type. Locks cover lookup
//!   and publication only.
//! - [`context`]: [`GenerationContext`] hands schemes owned `Arc` dependencies
//!   and generates what is missing.
//! - [`demand`]: [`HcsgDemand`], one subscription per bounds source, and the
//!   epoch that sessions advance.
//! - [`worker`]: [`HcsgWorker`], the thread that fills subscriptions.
//! - [`bounds`]: [`HcsgRegions<C>`], the boxes channel `C` wants values in,
//!   and the [`HcsgBounds`] producers that send them.
//! - [`generation`]: [`generation<C, T>`](generation::generation) keeps `T`
//!   warm within `C`'s regions, fire and forget.
//! - [`presentation`]: [`presentation<C, T>`](presentation::presentation)
//!   keeps one [`HcsgNode<T>`] host per published value within `C`'s regions.
//! - [`node`]: [`HcsgNode<T>`], forwarding the LOD scene traits to `T`.
//!
pub mod bounds;
pub mod node_store;
pub mod context;
pub mod demand;
pub mod generation;
pub mod node;
pub mod presentation;
pub mod runtime;
pub mod storage;
pub mod worker;

#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod tests;

pub use bounds::{Gated, HcsgBounds, HcsgBoundsPlugin, HcsgGate, HcsgRegions};
pub use context::{GenerationContext, GenerationScheme};
pub use demand::{HcsgDemand, Published, SubscriptionId};
pub use generation::GenerationPlugin;
pub use node::HcsgNode;
pub use presentation::{PresentationPlugin, RetiredHost};
pub use runtime::HcsgSystems;
pub use storage::{Busy, HcsgStorage, HcsgValue};
pub use worker::HcsgWorker;
