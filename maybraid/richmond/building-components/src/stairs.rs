//! Circulation stair scene components.
//!
//! Pipeline: [`geometry`] → [`geometry_components`] → named stair scene types.

pub mod geometry;
pub mod geometry_components;
pub mod rough_stone_spiral_stair;
pub mod rough_stone_straight_stair;
pub mod scene;
pub mod wood_straight_stair;

pub use geometry::*;
pub use geometry_components::StairComponent;
pub use rough_stone_spiral_stair::RoughStoneSpiralStair;
pub use rough_stone_straight_stair::RoughStoneStraightStair;
pub use scene::{rough_stone_stair, wood_stair};
pub use wood_straight_stair::WoodStraightStair;
