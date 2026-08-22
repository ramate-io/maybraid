//! Clear [`lod::LodLazyPending`] when SceneRef / merge instance mesh + MaterialRef
//! are both present.
//!
//! `lod` only waits on the marker. This crate knows `Mesh3d` and
//! [`material_ref::MaterialRefApplied`] (the SceneRef / MultiSceneMerge instance
//! path). Stamp the marker in vegetation BSN helpers; do not stamp SceneRef-only
//! or empty host shells.
//!
//! Add [`LodLazyRefsPlugin`] next to `SceneRefPlugin` and the material-ref plugin.

use bevy::prelude::{App, Children, Commands, Entity, Mesh3d, Plugin, Query, Update, With};

use lod::LodLazyPending;
use material_ref::MaterialRefApplied;

/// True when `root` or a descendant has both [`Mesh3d`] and [`MaterialRefApplied`].
pub fn subtree_has_mesh_and_material(
	root: Entity,
	children_q: &Query<&Children>,
	meshes: &Query<(), With<Mesh3d>>,
	applied: &Query<(), With<MaterialRefApplied>>,
) -> bool {
	if meshes.contains(root) && applied.contains(root) {
		return true;
	}
	let mut stack: Vec<Entity> = match children_q.get(root) {
		Ok(children) => children.iter().copied().collect(),
		Err(_) => return false,
	};
	while let Some(entity) = stack.pop() {
		if meshes.contains(entity) && applied.contains(entity) {
			return true;
		}
		if let Ok(kids) = children_q.get(entity) {
			stack.extend(kids.iter().copied());
		}
	}
	false
}

/// Drop [`LodLazyPending`] once self or a descendant is a fulfilled mesh.
pub fn clear_lod_lazy_when_mesh_and_material(
	mut commands: Commands,
	pending: Query<Entity, With<LodLazyPending>>,
	children_q: Query<&Children>,
	meshes: Query<(), With<Mesh3d>>,
	applied: Query<(), With<MaterialRefApplied>>,
) {
	for entity in &pending {
		if subtree_has_mesh_and_material(entity, &children_q, &meshes, &applied) {
			commands.entity(entity).remove::<LodLazyPending>();
		}
	}
}

/// Installs [`clear_lod_lazy_when_mesh_and_material`].
pub struct LodLazyRefsPlugin;

impl Plugin for LodLazyRefsPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, clear_lod_lazy_when_mesh_and_material);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::*;

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<Mesh>()
			.add_plugins(LodLazyRefsPlugin);
		app
	}

	fn mesh_handle(app: &mut App) -> Handle<Mesh> {
		app.world_mut()
			.resource_mut::<Assets<Mesh>>()
			.add(Mesh::from(Cuboid::from_length(1.0)))
	}

	#[test]
	fn stays_without_mesh() {
		let mut app = app();
		let entity = app.world_mut().spawn((LodLazyPending, MaterialRefApplied)).id();
		app.update();
		assert!(app.world().get::<LodLazyPending>(entity).is_some());
	}

	#[test]
	fn stays_without_material() {
		let mut app = app();
		let mesh = mesh_handle(&mut app);
		let entity = app.world_mut().spawn((LodLazyPending, Mesh3d(mesh))).id();
		app.update();
		assert!(app.world().get::<LodLazyPending>(entity).is_some());
	}

	#[test]
	fn clears_on_self_mesh_and_material() {
		let mut app = app();
		let mesh = mesh_handle(&mut app);
		let entity = app.world_mut().spawn((LodLazyPending, Mesh3d(mesh), MaterialRefApplied)).id();
		app.update();
		assert!(app.world().get::<LodLazyPending>(entity).is_none());
	}

	#[test]
	fn clears_on_descendant_mesh_and_material() {
		let mut app = app();
		let mesh = mesh_handle(&mut app);
		let root = app.world_mut().spawn((LodLazyPending, MaterialRefApplied)).id();
		app.world_mut().spawn((Mesh3d(mesh), MaterialRefApplied, ChildOf(root)));
		app.update();
		assert!(app.world().get::<LodLazyPending>(root).is_none());
	}

	#[test]
	fn stays_when_mesh_child_lacks_material() {
		let mut app = app();
		let mesh = mesh_handle(&mut app);
		let root = app.world_mut().spawn((LodLazyPending, MaterialRefApplied)).id();
		app.world_mut().spawn((Mesh3d(mesh), ChildOf(root)));
		app.update();
		assert!(app.world().get::<LodLazyPending>(root).is_some());
	}
}
