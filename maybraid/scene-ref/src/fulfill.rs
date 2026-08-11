//! Fulfill [`SceneRefRoot`] / [`MultiSceneMergeRoot`] → [`WorldAssetRoot`].

use bevy::asset::AssetServer;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::prelude::{Assets, Commands, Entity, Query, Res, ResMut, StandardMaterial, Without};
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};

use crate::handles::SceneRefHandles;
use crate::multi_merge::{MultiSceneMergeHandles, MultiSceneMergeRoot};
use crate::scene_ref::SceneRefRoot;

pub(crate) fn fulfill_scene_ref_roots(
	mut commands: Commands,
	query: Query<(Entity, &SceneRefRoot), Without<WorldAssetRoot>>,
	mut handles: ResMut<SceneRefHandles>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	type_registry: Res<AppTypeRegistry>,
) {
	for (entity, root) in &query {
		if let Some(handle) = handles.try_resolve(
			&root.0,
			&asset_server,
			&mut world_assets,
			&mut meshes,
			&type_registry,
		) {
			commands.entity(entity).insert(WorldAssetRoot(handle));
		}
	}
}

pub(crate) fn fulfill_multi_scene_merge_roots(
	mut commands: Commands,
	query: Query<(Entity, &MultiSceneMergeRoot), Without<WorldAssetRoot>>,
	mut scene_handles: ResMut<SceneRefHandles>,
	mut merge_handles: ResMut<MultiSceneMergeHandles>,
	asset_server: Res<AssetServer>,
	mut world_assets: ResMut<Assets<WorldAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	type_registry: Res<AppTypeRegistry>,
) {
	for (entity, root) in &query {
		if let Some(handle) = merge_handles.try_resolve(
			&root.0,
			&mut scene_handles,
			&asset_server,
			&mut world_assets,
			&mut meshes,
			&mut materials,
			&type_registry,
		) {
			commands.entity(entity).insert(WorldAssetRoot(handle));
		}
	}
}
