//! [`LodViewer`] marker: driver nodes for probe / chunk systems.

use bevy::prelude::*;

use crate::lod_ref::LodNode;

/// Marker: this [`LodNode`] is an LOD viewer (cameras, fly-cams, ground anchors, …).
///
/// Probe and chunk systems query `(…, With<LodViewer>)` for pose and fold every
/// match (min distance / max detail). There is no mirrored [`Resource`] copy.
#[derive(Debug, Clone, Copy, Default, Component)]
#[require(LodNode)]
pub struct LodViewer;
