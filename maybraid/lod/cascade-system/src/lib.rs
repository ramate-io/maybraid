//! Bevy-facing systems and wiring for the LOD cascade ([RFC-154](https://github.com/ramate-io/maybraid/issues/157)).
//!
//! Most of the generalized LOD RFC describes how this layer schedules work, tracks chunks in-world, and integrates with the renderer. Core cascade math and state live in [`lod_cascade`] so non-Bevy callers can reuse it.
//!
//! # Three layers
//!
//! 1. **[`cascade_production`]** — [`produce_cascade`] maintains each producer’s [`CascadeProduction`] table and chunk hierarchy from track bounds; optional [`garbage_collect_requirement_signals`] clears transient signal entities.
//! 2. **[`chunk_tracker`]** — Implement [`ChunkTracker`] and register [`ChunkTrackerPlugin`] to react when [`RequirementSignal`] updates on footprint signal entities (LOD / streaming hooks).
//! 3. **[`chunk_entity_tracker`]** — Implement [`ChunkEntityPosition`] for your managed payload and register [`ChunkEntityTrackerPlugin`] so entities parented under [`CascadeChunk`] children follow the best overlapping chunk when their bounds move.
//!
//! Use the same [`CascadeProductionSource`] type across plugins (for example [`StandardFlow`] with a tag type, [`StandardRequirement`], and `()` for every-frame scheduling) so each layer listens to the same flow.
//!
//! # Example
//!
//! ```
//! # use bevy::prelude::*;
//! # use bevy::math::bounding::Aabb3d;
//! # use lod_cascade::Cascade;
//! # use lod_cascade_system::{
//! #     CascadeProduction, CascadeProductionPlugin, ChunkEntityPosition, ChunkEntityTrackerPlugin,
//! #     ChunkTracker, ChunkTrackerPlugin, MarkedBounds, RequirementSignal, StandardFlow,
//! #     StandardRequirement, TrackBounds,
//! # };
//! #
//! # #[derive(Debug)]
//! # struct GameTag;
//! #
//! # type GameFlow = StandardFlow<GameTag, StandardRequirement, ()>;
//! #
//! # #[derive(Component, Clone)]
//! # struct ManagedBounds {
//! #     prev: Option<TrackBounds>,
//! #     cur: TrackBounds,
//! # }
//! #
//! # impl ChunkEntityPosition<GameFlow> for ManagedBounds {
//! #     fn previous(&self) -> Option<TrackBounds> {
//! #         self.prev
//! #     }
//! #     fn current(&self) -> TrackBounds {
//! #         self.cur
//! #     }
//! # }
//! #
//! # struct MyChunkTracker;
//! #
//! # impl ChunkTracker<GameFlow> for MyChunkTracker {
//! #     fn react(_commands: &mut Commands, _chunk: lod_cascade::Chunk, _signal: RequirementSignal) {}
//! # }
//! let mut app = App::new();
//! app.add_plugins(MinimalPlugins);
//! app.add_plugins((
//!     CascadeProductionPlugin::<GameFlow>::default(),
//!     ChunkTrackerPlugin::<MyChunkTracker, GameFlow>::default(),
//!     ChunkEntityTrackerPlugin::<ManagedBounds, GameFlow>::default(),
//! ));
//!
//! app.world_mut().spawn((
//!     CascadeProduction::<GameFlow>::new(Cascade::new(Vec3::ONE, 0, None)),
//!     MarkedBounds::<GameTag>::new(Aabb3d::new(Vec3::ZERO, Vec3::splat(50.0))),
//!     StandardRequirement::default(),
//! ));
//! ```

pub mod cascade_production;

pub use cascade_production::{
	garbage_collect_requirement_signals, produce_cascade, CascadeChunk, CascadePosition,
	CascadeProduction, CascadeProductionPlugin, CascadeProductionSignalMarker,
	CascadeProductionSource, CascadeTable, MarkedBounds, RequirementBuilder, RequirementSignal,
	StandardFlow, StandardRequirement, TrackBounds,
};

pub mod chunk_tracker;

pub use chunk_tracker::{track_chunks, ChunkTracker, ChunkTrackerPlugin};

pub mod chunk_entity_tracker;

pub use chunk_entity_tracker::{
	select_best_overlapping_chunk, track_chunk_entities, ChunkEntityPosition,
	ChunkEntityTrackerPlugin,
};

#[cfg(test)]
pub mod tests;
