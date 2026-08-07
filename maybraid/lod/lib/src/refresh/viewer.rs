//! [`LodViewer`] marker (a [`super::node::LodNode`] that also feeds [`LodViewerState`]).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::lod_ref::LodRef;

use super::node::LodNode;

/// Marker: this [`LodNode`] is the primary viewer for [`LodViewerState`] probes.
#[derive(Debug, Clone, Copy, Default, Component)]
#[require(LodNode)]
pub struct LodViewer;

/// Cached primary-viewer pose for systems that still take a single [`LodRef`].
///
/// Prefer querying [`LodNode`]s directly in new code. Core keeps this in sync from
/// the sole [`LodViewer`] node.
#[derive(Resource, Debug)]
pub struct LodViewerState {
	pub entity: Entity,
	pub previous: Transform,
	pub current: Transform,
	/// True when `current.translation` differs from `previous` this frame.
	pub translated: bool,
}

impl Default for LodViewerState {
	fn default() -> Self {
		Self {
			entity: Entity::PLACEHOLDER,
			previous: Transform::IDENTITY,
			current: Transform::IDENTITY,
			translated: false,
		}
	}
}

impl LodViewerState {
	/// Borrowed [`LodRef`] for `bounds` this frame (no clones of transforms).
	pub fn lod_ref<'a>(&'a self, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: self.entity,
			previous_transform: &self.previous,
			current_transform: &self.current,
			bounds,
		}
	}
}
