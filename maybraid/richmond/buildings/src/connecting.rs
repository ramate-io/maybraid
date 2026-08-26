//! Connectors between mapped shell openings (sibling of [`crate::shells`]).
//!
//! [`hall`] joins two same-storey wall quads with a one-kink [`crate::Tube`].
//! [`stairwell`] joins two horizontal shaft faces (a vertical well): owned
//! thin floor slabs (run-in / optional upper landing) and a fitted stair
//! flight (`with_flight`; no well walls).

mod geom;
pub mod hall;
pub mod stairwell;

pub use hall::{ConnectingHall, HallOpening};
pub use stairwell::{
	ConnectingStairwell, StairwellOpening, TreadEnd, GOING_RATIO_DEFAULT, RUN_IN_M,
	SLAB_THICKNESS_M, TREAD_FILL_DEFAULT,
};
