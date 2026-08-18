//! Patch [`LodHostBounds`] and backend volumes from [`LodScene::scene_bounds`].

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::scene::host::LodSceneHost;
use crate::scene::refresh::{LodHostBounds, LodRefreshSystems};
use crate::scene::LodScene;

/// Converts a local [`Aabb3d`] into a backend-specific volume [`Bundle`].
///
/// Used by [`PatchSceneBounds`] so region indexes (Avian, etc.) can materialize
/// searchable volumes without `LodScene` depending on a physics crate.
pub trait LodSceneBoundsMarshaller: Send + Sync + 'static {
	type Volume: Bundle;

	fn volume_from_bounds(bounds: Aabb3d) -> Self::Volume;
}

/// Writes [`LodHostBounds`] from [`LodScene::scene_bounds`] and inserts
/// [`LodSceneBoundsMarshaller::Volume`] on the host.
pub struct PatchSceneBounds<T, M>
where
	T: Component + LodScene + 'static,
	M: LodSceneBoundsMarshaller,
{
	_marker: PhantomData<fn() -> (T, M)>,
}

impl<T, M> Default for PatchSceneBounds<T, M>
where
	T: Component + LodScene + 'static,
	M: LodSceneBoundsMarshaller,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, M> Plugin for PatchSceneBounds<T, M>
where
	T: Component + LodScene + 'static,
	M: LodSceneBoundsMarshaller,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<crate::scene::refresh::LodRefreshCorePlugin>() {
			app.add_plugins(crate::scene::refresh::LodRefreshCorePlugin);
		}
		app.add_systems(
			Update,
			patch_scene_bounds::<T, M>.before(LodRefreshSystems::ProduceLevels),
		);
	}
}

fn patch_scene_bounds<T, M>(
	mut commands: Commands,
	hosts: Query<(Entity, &T), (With<LodSceneHost>, Or<(Added<T>, Changed<T>)>)>,
) where
	T: Component + LodScene + 'static,
	M: LodSceneBoundsMarshaller,
{
	for (entity, scene) in &hosts {
		let bounds = scene.scene_bounds();
		commands
			.entity(entity)
			.insert((LodHostBounds(bounds), M::volume_from_bounds(bounds)));
	}
}
