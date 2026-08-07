//! Viewer pose → [`LodViewerState`] for ephemeral [`crate::LodRef`]s.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::lod_ref::LodRef;

/// Marker: this entity's [`Transform`] drives fine-phase [`LodRef`] construction.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodViewer;

/// Previous / current viewer pose for ephemeral [`LodRef`]s (no owned component).
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

/// Copy the primary [`LodViewer`] transform into [`LodViewerState`].
pub fn track_lod_viewer(
	viewers: Query<(Entity, &Transform), With<LodViewer>>,
	mut state: ResMut<LodViewerState>,
) {
	state.translated = false;
	let Ok((entity, transform)) = viewers.single() else {
		return;
	};
	state.previous = state.current;
	state.current = *transform;
	state.entity = entity;
	state.translated =
		(state.previous.translation - state.current.translation).length_squared() > 1e-8;
}
