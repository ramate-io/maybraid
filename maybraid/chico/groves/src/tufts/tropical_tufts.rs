//! Tropical Tufts for Chico vegetation ([RFC-183 §3.4.4.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/04-well-known-tufts-groves/05-tropical-tufts/README.md)).

mod cell;
mod definition;
mod palm_bush;
mod tuft;

#[cfg(feature = "render")]
mod frontend;

#[cfg(feature = "render")]
pub mod render_item;

pub use cell::TropicalTuftsCell;
pub use definition::TropicalTuftsDefinition;
pub use palm_bush::TropicalPalmBush;
pub use tuft::TropicalTuftClump;

#[cfg(feature = "render")]
pub use frontend::TropicalTuftsGroveFrontend;

#[cfg(feature = "render")]
pub use render_item::{TropicalTufts, TropicalTuftsStd};
