//! Building envelope shells composed over paneling primitives.

pub mod arc_floor;
pub mod arc_tower;
pub mod connecting_hall;
pub mod pitched_rectangular_roof;
pub mod rectangular_pitched_roof_complex;
pub mod trazaloid;

pub use arc_floor::{ArcFloor, ArcFloorParams, ArcFloorSlab};
pub use arc_tower::{ArcTower, ArcTowerParams};
pub use connecting_hall::ConnectingHall;
pub use pitched_rectangular_roof::{PitchedRoof, PitchedRoofParams, RoofHalf};
pub use rectangular_pitched_roof_complex::{
	EndCap, Overhang, RectangularPitchedRoofComplex, RectangularPitchedRoofComplexParams,
	RidgeJunction, ValleySegment,
};
pub use trazaloid::{Trazaloid, TrazaloidParams, TrazaloidSide, TrazaloidSlab};
