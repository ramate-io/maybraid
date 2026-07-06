//! LOD spatial loading sketch.
//!
//! Main idea:
//!
//! - `SpatialIndex<T>` owns spatial truth.
//! - `GeneratingSpatialIndex<T>` materializes missing values.
//! - `SceneLoader` is middleware over generation that can also spawn/heal.
//!
//! Pitfall avoided:
//! spawning from `insert()` is too implicit. It can make descendant generation
//! accidentally spawn visuals and can miss moved assets that need healing first.

mod generation;
mod id;
mod loader;
mod scene;
mod spatial_index;

#[cfg(test)]
mod tests;

pub use generation::{BuildWithIdLod, GeneratingSpatialIndex, MaterializeStatus};
pub use id::{Bytes, Cell, Id, OriginCell, OriginalId, StorageStatus, TrackedId};
pub use loader::{Materialize, SceneLoader};
pub use scene::{LodScene, ScenePatchStatus, SceneSpawner};
pub use spatial_index::{BaseSpatialIndex, OriginalIds, SpatialIndex};
