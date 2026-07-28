//! Roof / cap scene components.
//!
//! Pipeline: [`geometry`] → [`geometry_components`] → material variant modules.

pub mod geometry;
pub mod geometry_components;
pub mod rough_stonework;
pub mod scene;
pub mod wood;

pub use geometry::*;
pub use geometry_components::RoofComponent;
pub use rough_stonework::*;
pub use scene::roof_scene;
pub use wood::*;
