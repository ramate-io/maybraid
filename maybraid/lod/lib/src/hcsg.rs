//! Hierarchical CSG storage and generation wiring
//! ([#1005](https://github.com/ramate-io/maybraid/issues/1005)).
//!
//! - [`storage`]: [`HcsgStorage`], one typed [`NodeStore`] per generated type
//!   with a `gimme` spatial index. It implements [`crate::gen::SpatialIndex`]
//!   for every type, so a layer writes [`crate::gen::GenerationScheme`]s and
//!   no store or marshalling adapter.
//! - [`generate`]: typed producers publish [`GenerationBounds`] on their own
//!   channel; [`GenerateOn<P, T>`] subscribes `T` to producer `P`. Root inputs
//!   enter through [`Seed`].
//!
//! Generation stays synchronous and pose-free. Presentation keeps the LOD
//! vocabulary ([`crate::presentation`], [`crate::scene`]).
//!
//! [`shared`] is the threaded runtime replacing both, alongside them until
//! the layers move over.

pub mod generate;
pub mod schedule;
pub mod shared;
pub mod storage;

#[cfg(test)]
mod tests;

pub use generate::{
	generate, produce_from_nodes, universal_bounds, ChannelTimeBudget, CurrentBounds, GenerateOn,
	GenerateQueue, GenerationBounds, GenerationChannelPlugin, GenerationProducer,
	GenerationRequest, HcsgSeedSystems, ProduceFromNodes, Reseeded, Seed,
};
pub use schedule::{LodGenerateBudget, LodGenerateSystems, LodGenerateTimeBudget, LodGenerated};
pub use storage::{HcsgNode, HcsgStorage, NodeStore, StoredEntry, DEFAULT_BASE_SCALE};
