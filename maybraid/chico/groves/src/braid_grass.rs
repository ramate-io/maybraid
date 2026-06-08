//! Braid Grass for Chico vegetation ([RFC-183 §3.4.5.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-04-cellular-groves/05-well-known-understory-groves/01-braid-grass/README.md)).

mod braid_grass;
mod cell;
mod definition;

#[cfg(feature = "render")]
mod sample;

#[cfg(feature = "render")]
pub mod render_item;

pub use braid_grass::BraidGrassClump;
pub use cell::BraidGrassCell;
pub use definition::BraidGrassDefinition;

#[cfg(feature = "render")]
pub use render_item::{BraidGrass, BraidGrassRenderRule, BraidGrassStd};

#[cfg(feature = "render")]
pub use sample::{blade_tuft_shape_from, unit_from_position};
