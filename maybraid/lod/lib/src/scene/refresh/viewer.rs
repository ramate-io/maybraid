//! [`LodViewer`] marker: primary driver node for probe / chunk systems.

use bevy::prelude::*;

use crate::lod_ref::LodNode;

/// Marker: this [`LodNode`] is the primary viewer (cameras, fly-cams, …).
///
/// Probe and chunk systems query `(…, With<LodViewer>)` for pose; there is no
/// mirrored [`Resource`] copy.
#[derive(Debug, Clone, Copy, Default, Component)]
#[require(LodNode)]
pub struct LodViewer;
