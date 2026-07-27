//! Floor slab scene components.
//!
//! Pipeline: [`geometry`] → [`geometry_components`] → named floor scene types.

pub mod geometry;
pub mod geometry_components;
pub mod rough_stone_floor_arc_fill;
pub mod rough_stone_floor_rectangle;
pub mod rough_stone_floor_struct_fill;
pub mod scene;
pub mod wood_floor_arc_fill;
pub mod wood_floor_rectangle;
pub mod wood_floor_struct_fill;

pub use geometry::*;
pub use geometry_components::FloorComponent;
pub use rough_stone_floor_arc_fill::RoughStoneFloorArcFill;
pub use rough_stone_floor_rectangle::RoughStoneFloorRectangle;
pub use rough_stone_floor_struct_fill::RoughStoneFloorStructFill;
pub use scene::{rough_stone_floor, wood_floor};
pub use wood_floor_arc_fill::WoodFloorArcFill;
pub use wood_floor_rectangle::WoodFloorRectangle;
pub use wood_floor_struct_fill::WoodFloorStructFill;
