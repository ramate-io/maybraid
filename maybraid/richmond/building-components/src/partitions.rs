//! Wall / partition scene components (linear, angular, and header variants).
//!
//! Pipeline: [`geometry`] → [`geometry_components`] → [`rough_stonework`] scene types.

pub mod geometry;
pub mod geometry_components;
pub mod rough_stonework;
pub mod scene;

pub use geometry::*;
pub use geometry_components::{decompose_arc_sweep, ArcKit, WallComponent};
pub use rough_stonework::*;
pub use scene::{rough_stone_wall, wall_component_scene};
