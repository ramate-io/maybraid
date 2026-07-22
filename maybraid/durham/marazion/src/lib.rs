//! Marazion watershed stamps ([RFC-127](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds)).
//!
//! Pure stamp / layout construction — no LOD `GenerationScheme` wiring.
//! Durham models consume [`jersey_terrain_stamps::JerseyModulation`] and
//! [`WaterFill`] outputs after pre-watershed terrain is composed.
//!
//! Pocket hierarchy: [`pre_pocket`] → [`pocket_cell`] guillotine → [`lake`] / [`stream`] leaves.

pub mod fill;
pub mod lake;
pub mod noise;
pub mod pocket_cell;
pub mod polyline;
pub mod pre_pocket;
pub mod stream;

pub use fill::{WaterFill, WaterSurface};
pub use lake::{shelf_base_height, Lake, LakeBandBudget, LakeParams};
pub use pocket_cell::{guillotine_partition, PocketGuillotineParams};
pub use polyline::{closest_on_polyline, grade_along_polyline, ClosestOnPolyline};
pub use pre_pocket::{
	PrePocket, PrePocketParams, DEFAULT_POCKET_PITCHES, DEFAULT_POCKET_PITCHES_HIGH,
	DEFAULT_POCKET_PITCHES_LOW, DEFAULT_PRE_POCKET_PITCH, DEFAULT_PRE_POCKET_PITCH_LOW,
};
pub use stream::{Stream, StreamBandBudget, StreamParams};
