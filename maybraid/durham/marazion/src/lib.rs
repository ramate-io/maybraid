//! Marazion watershed stamps ([RFC-127](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds)).
//!
//! Pure stamp / layout construction — no LOD `GenerationScheme` wiring.
//! Durham models consume [`jersey_terrain_stamps::JerseyModulation`] and
//! [`WaterFill`] outputs after pre-watershed terrain is composed.
//!
//! Pocket hierarchy: [`pre_pocket`] → [`pocket_cell`] guillotine → [`lake`] /
//! [`stream`] / [`bog`] **plans**. Each plan realizes a
//! [`complex::WatershedDepressionComplex`]; Durham compiles that graph when
//! pulling into terrain (per-complex apron → carve → backfill order).

pub mod apron;
pub mod backfill;
pub mod bog;
pub mod complex;
pub mod depression;
pub mod fill;
pub mod lake;
pub mod noise;
pub mod pocket_cell;
pub mod polyline;
pub mod pre_pocket;
pub mod stream;

pub use apron::{ApronNoiseSalts, WatershedApronParams};
pub use backfill::{
	BasinBackfillParams, WatershedBackfill, WatershedBackfillKind,
};
pub use bog::{Bog, BogBasinFill, BogParams};
pub use complex::{
	CompiledWatershed, WatershedApronShelf, WatershedDepressionComplex, WatershedEdge,
	WatershedEdgeId, WatershedNode, WatershedNodeId,
};
pub use depression::{WatershedDepression, WatershedDepressionKind};
pub use fill::{WaterFill, WaterSurface};
pub use lake::{shelf_base_height, Lake, LakeBandBudget, LakeParams};
pub use pocket_cell::{guillotine_partition, PocketGuillotineParams};
pub use polyline::{closest_on_polyline, grade_along_polyline, ClosestOnPolyline};
pub use pre_pocket::{
	PrePocket, PrePocketParams, DEFAULT_POCKET_PITCHES, DEFAULT_POCKET_PITCHES_HIGH,
	DEFAULT_POCKET_PITCHES_LOW, DEFAULT_PRE_POCKET_PITCH, DEFAULT_PRE_POCKET_PITCH_LOW,
};
pub use stream::{Stream, StreamBandBudget, StreamParams};
