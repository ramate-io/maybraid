//! Patch [`LodHostBounds`] and backend volumes from [`LodScene::scene_bounds`].
//!
//! Scene-specific observers compute bounds only when the scene component is
//! inserted. One marshaller system per backend reacts to changed untyped bounds,
//! avoiding one full query system per scene type every frame.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::scene::host::LodSceneHost;
use crate::scene::refresh::{LodHostBounds, LodRefreshSystems};
use crate::scene::SemanticLodScene;

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
	T: Component + SemanticLodScene + 'static,
	M: LodSceneBoundsMarshaller,
{
	_marker: PhantomData<fn() -> (T, M)>,
}

impl<T, M> Default for PatchSceneBounds<T, M>
where
	T: Component + SemanticLodScene + 'static,
	M: LodSceneBoundsMarshaller,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, M> Plugin for PatchSceneBounds<T, M>
where
	T: Component + SemanticLodScene + 'static,
	M: LodSceneBoundsMarshaller,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<crate::scene::refresh::LodRefreshCorePlugin>() {
			app.add_plugins(crate::scene::refresh::LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<PatchSceneBoundsMarshallerPlugin<M>>() {
			app.add_plugins(PatchSceneBoundsMarshallerPlugin::<M>::default());
		}
		app.add_observer(update_scene_bounds::<T>);
	}
}

fn update_scene_bounds<T>(insert: On<Insert, T>, mut commands: Commands, scenes: Query<&T>)
where
	T: Component + SemanticLodScene + 'static,
{
	let Ok(scene) = scenes.get(insert.entity) else {
		return;
	};
	if let Ok(mut entity) = commands.get_entity(insert.entity) {
		entity.insert(LodHostBounds(scene.scene_bounds()));
	}
}

struct PatchSceneBoundsMarshallerPlugin<M: LodSceneBoundsMarshaller> {
	_marker: PhantomData<fn() -> M>,
}

impl<M: LodSceneBoundsMarshaller> Default for PatchSceneBoundsMarshallerPlugin<M> {
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<M: LodSceneBoundsMarshaller> Plugin for PatchSceneBoundsMarshallerPlugin<M> {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			patch_marshaled_bounds::<M>.before(LodRefreshSystems::ProduceLevels),
		);
	}
}

fn patch_marshaled_bounds<M: LodSceneBoundsMarshaller>(
	mut commands: Commands,
	hosts: Query<(Entity, &LodHostBounds), (With<LodSceneHost>, Changed<LodHostBounds>)>,
) {
	for (entity, bounds) in &hosts {
		commands.entity(entity).insert(M::volume_from_bounds(bounds.0));
	}
}
