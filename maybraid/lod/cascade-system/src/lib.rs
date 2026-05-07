//! Bevy-facing systems and wiring for the LOD cascade ([RFC-154](https://github.com/ramate-io/maybraid/issues/157)).
//!
//! Most of the generalized LOD RFC describes how this layer schedules work, tracks chunks in-world, and integrates with the renderer. Core cascade math and state live in [`lod_cascade`] so non-Bevy callers can reuse it.

pub mod cascade_production;

pub use cascade_production::{
	garbage_collect_requirement_signals, produce_cascade, CascadeChunk, CascadePosition,
	CascadeProduction, CascadeProductionPlugin, CascadeProductionSignalMarker,
	CascadeProductionSource, CascadeTable, MarkedBounds, RequirementBuilder, RequirementSignal,
	StandardFlow, StandardRequirement, TrackBounds,
};
