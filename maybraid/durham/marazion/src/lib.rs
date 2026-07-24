//! Marazion watershed stamps ([RFC-127](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds)).
//!
//! Pure stamp / layout construction — no LOD `GenerationScheme` wiring.
//! Durham models consume [`HydrologyComplex`] and [`WaterFill`] outputs
//! after pre-watershed terrain is composed.
//!
//! Module layout:
//! - [`authored`]: pocket hierarchy and leaf plans (`lake` / `stream` / `bog` / …)
//! - [`primitive`]: blendable nodes, complexes, hydro fields, and water fill
//!
//! Pocket hierarchy: [`authored::pre_pocket`] → [`authored::pocket_cell`] guillotine →
//! leaf plans. Each plan emits [`primitive::node::HydrologyNode`]s into a
//! [`primitive::complex::HydrologyComplex`]; Durham gathers those into cellular
//! complexes for terrain modulation.

pub mod authored;
pub mod primitive;

pub use authored::apron::{ApronNoiseSalts, WatershedApronParams, TARGET_RIM_WIDTH};
pub use authored::bog::{Bog, BogBasinFill, BogParams};
pub use authored::lake::{shelf_base_height, Lake, LakeBandBudget, LakeParams};
pub use authored::pocket_cell::{guillotine_partition, PocketGuillotineParams};
pub use authored::polyline::{closest_on_polyline, grade_along_polyline, ClosestOnPolyline};
pub use authored::pre_pocket::{
	PrePocket, PrePocketParams, DEFAULT_POCKET_PITCHES, DEFAULT_POCKET_PITCHES_HIGH,
	DEFAULT_POCKET_PITCHES_LOW, DEFAULT_PRE_POCKET_PITCH, DEFAULT_PRE_POCKET_PITCH_LOW,
};
pub use authored::stream::{Stream, StreamBandBudget, StreamParams};
pub use authored::streams_graph::{StreamsGraph, StreamsGraphParams};
pub use primitive::backfill::{BasinBackfillParams, WatershedBackfill, WatershedBackfillKind};
pub use primitive::complex::{CompiledWatershed, HydrologyComplex};
pub use primitive::fill::{WaterFill, WaterSurface};
pub use primitive::hydro::{
	primitives_from_polyline, ComplexApronParams, CorrectionStage, Ellipse, FootprintIndex,
	HydroElevation, HydroFootprint, HydroPrimitive, RadialBowl, ReachProfile, ReachSegment,
	DEFAULT_RIM_UPLIFT_CAP, SURFACE_SMOOTHMIN_K,
};
pub use primitive::node::{nodes_from_polyline, HydrologyNode, HydroParameters};
