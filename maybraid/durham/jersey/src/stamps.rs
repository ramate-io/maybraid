//! Jersey stamp families ([RFC-105 §3.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)).

pub mod valley_basin;

pub use valley_basin::{
	ValleyBasin, ValleyBasinParams, ValleyCrossSection, ValleyFloorKind,
};
