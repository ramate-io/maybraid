//! Bevy-facing systems and wiring for the LOD cascade ([RFC-154](https://github.com/ramate-io/maybraid/issues/157)).
//!
//! Most of the generalized LOD RFC describes how this layer schedules work, tracks chunks in-world, and integrates with the renderer. Core cascade math and state live in [`lod_cascade`] so non-Bevy callers can reuse them.

pub mod cascade_production;

pub use cascade_production::{
	garbage_collect_requirement_signals, produce_cascade, CascadeBounds, CascadeChunk,
	CascadePosition, CascadeProduction, CascadeProductionPlugin,
	CascadeProductionSignalMarker, CascadeProductionSource, CascadeTable, RequirementBuilder,
	RequirementSignal, StandardBounds, StandardFlow, StandardMarker, StandardRequirement,
};
