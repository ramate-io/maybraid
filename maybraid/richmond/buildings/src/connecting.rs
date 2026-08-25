//! Connectors between mapped shell openings (sibling of [`crate::shells`]).
//!
//! [`hall`] joins two same-storey wall quads with a one-kink [`crate::Tube`].
//! [`stairwell`] joins two horizontal shaft faces (a vertical well): owned
//! run-in floor and a fitted stair flight (no well walls).

mod geom;
pub mod hall;
pub mod stairwell;

pub use hall::{ConnectingHall, HallOpening};
pub use stairwell::{ConnectingStairwell, StairwellOpening, RUN_IN_M};
