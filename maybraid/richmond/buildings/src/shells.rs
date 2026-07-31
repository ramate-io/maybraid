//! Building envelope shells composed over paneling primitives.

pub mod connecting_hall;
pub mod trazaloid;

pub use connecting_hall::{ConnectingHall, ConnectingHallEndpoint};
pub use trazaloid::{Trazaloid, TrazaloidDoors, TrazaloidParams, TrazaloidSlab};
