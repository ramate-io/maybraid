//! Floor slab scene components.
//!
//! Pipeline: [`geometry`] → [`geometry_components`] → material variant modules.

pub mod geometry;
pub mod geometry_components;
pub mod rough_stonework;
pub mod scene;
pub mod wood;

pub use geometry::*;
pub use geometry_components::FloorComponent;
pub use rough_stonework::*;
pub use scene::{rough_stone_floor, wood_floor};
pub use wood::*;
