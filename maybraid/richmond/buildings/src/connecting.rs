//! Connectors between mapped shell openings (sibling of [`crate::shells`]).
//!
//! [`hall`] joins two same-storey wall quads with a one-kink [`crate::Tube`].
//! [`stairwell`] joins a floor-space anchor opening to an upper landing: owned
//! run-in floor and a fitted stair flight (no well walls).

mod geom;
pub mod hall;
pub mod stairwell;

pub use hall::{ConnectingHall, HallOpening};
pub use stairwell::{ConnectingStairwell, StairwellOpening, RUN_IN_M};
