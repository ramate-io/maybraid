//! The top-level terrain model.
//! Usually this is what other layers will request when trying to figure out elevation.

use lod::gen::GenerationScheme;

#[derive(Debug, Clone, Component)]
pub struct Terrain;
