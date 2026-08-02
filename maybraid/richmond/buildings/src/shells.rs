//! Building envelope shells composed over paneling primitives.

pub mod arc_floor;
pub mod arc_tower;
pub mod connecting_hall;
pub mod trazaloid;

pub use arc_floor::{ArcFloor, ArcFloorParams, ArcFloorSlab};
pub use arc_tower::{ArcTower, ArcTowerParams};
pub use connecting_hall::ConnectingHall;
pub use trazaloid::{
	side_passage_opening, Trazaloid, TrazaloidParams, TrazaloidSide, TrazaloidSlab,
};
