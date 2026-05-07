//! Cascade logic for generalized LOD: chunk tracking, refinement decisions, and related algorithms without a game engine.
//!
//! Use this crate when you want RFC-154-style cascade behavior without pulling in Bevy ECS or `App`; math types come from `bevy_math`.

mod cascade;
mod chunk;

pub use bevy_math::bounding::Aabb3d;
pub use cascade::{Cascade, GridConfig};
pub use chunk::Chunk;
