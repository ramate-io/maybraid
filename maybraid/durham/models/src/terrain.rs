//! The top-level terrain model.
//! Usually this is what other layers will request when trying to figure out elevation.

use lod::gen::GenerationScheme;

#[derive(Debug, Clone, Component)]
pub struct Terrain;

/// Note that, at this point, Terrain doesn't have any dependencies on other layers,
/// so ther requirement is simply that the spatial index be able to store and retrieve the terrain.
///
/// Otherwise, we might have something like:
///
/// impl <S: GeneratingSpatialIndex<HydrologyGraph>> GenerationScheme<S> for Terrain {}
impl<S: Sized> GenerationScheme<S> for Terrain {}
