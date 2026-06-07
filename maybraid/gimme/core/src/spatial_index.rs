//! Multi-resolution spatial index ([RFC-142 §3.1]).

mod grid;
mod index;
mod store;
mod typed;

pub use grid::{BaseScale, Level};
pub use index::SpatialIndex;
pub use store::{HashMapStore, SpatialId, SpatialStore};
pub use typed::TypedIndex;
