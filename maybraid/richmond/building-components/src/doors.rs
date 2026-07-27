//! Door frame and leaf scene components.
//!
//! Pipeline: [`geometry`] → [`geometry_components`] → named door scene types.
//! Frames expand into partition wall kit pieces.

pub mod geometry;
pub mod geometry_components;
pub mod scene;
pub mod wood_door_leaf;

pub use geometry::*;
pub use geometry_components::DoorComponent;
pub use scene::door_scene;
pub use wood_door_leaf::WoodDoorLeaf;
