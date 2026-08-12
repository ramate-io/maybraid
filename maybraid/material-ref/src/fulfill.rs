//! Fulfill [`MaterialRefRoot`] via a generic [`MaterialLib`] [`SystemParam`].

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::{
	Added, App, ChildOf, Commands, Entity, IntoScheduleConfigs, Mesh3d, Plugin, Query, Update,
	With, Without,
};

use crate::lib_trait::MaterialLib;
use crate::material_ref::{
	MaterialRef, MaterialRefApplied, MaterialRefRoot, PropagateToDescendants,
};

/// For each [`MaterialRefRoot`] without [`MaterialRefApplied`], ask `L` to insert a material
/// (or register a propagating root).
pub fn fulfill_material_ref_roots<L>(
	mut commands: Commands,
	query: Query<(Entity, &MaterialRefRoot), Without<MaterialRefApplied>>,
	propagate: Query<(), With<PropagateToDescendants>>,
	meshes: Query<(), With<Mesh3d>>,
	lib: StaticSystemParam<L>,
) where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	let mut lib = lib.into_inner();
	for (entity, root) in &query {
		let propagate = propagate.contains(entity);
		let has_mesh = meshes.contains(entity);
		if propagate && !has_mesh {
			// Meshes arrive later under WorldAsset; only mark the root seen.
			commands.entity(entity).insert(MaterialRefApplied);
			continue;
		}
		lib.fulfill(entity, &root.0, &mut commands);
		commands.entity(entity).insert(MaterialRefApplied);
	}
}

/// When [`PropagateToDescendants`] is set, fulfill newly added `Mesh3d` entities under the root.
pub fn fulfill_material_ref_descendants<L>(
	mut commands: Commands,
	added_meshes: Query<Entity, (Added<Mesh3d>, Without<MaterialRefApplied>)>,
	parents: Query<&ChildOf>,
	roots: Query<&MaterialRefRoot, With<PropagateToDescendants>>,
	lib: StaticSystemParam<L>,
) where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	let mut lib = lib.into_inner();
	for entity in &added_meshes {
		let Some(material_ref) = propagating_material_ref(entity, &parents, &roots) else {
			continue;
		};
		lib.fulfill(entity, &material_ref, &mut commands);
		commands.entity(entity).insert(MaterialRefApplied);
	}
}

fn propagating_material_ref(
	mut entity: Entity,
	parents: &Query<&ChildOf>,
	roots: &Query<&MaterialRefRoot, With<PropagateToDescendants>>,
) -> Option<MaterialRef> {
	loop {
		if let Ok(root) = roots.get(entity) {
			return Some(root.0.clone());
		}
		entity = parents.get(entity).ok()?.parent();
	}
}

/// Installs fulfill for material lib `L` (a [`SystemParam`] whose item implements [`MaterialLib`]).
///
/// Parallel to [`lod::LodSceneRefreshLevelsPlugin`]’s index parameter: pass the concrete
/// `#[derive(SystemParam)]` type (e.g. [`crate::StandardMaterialLib`]).
///
/// Resources the lib needs (caches, etc.) must be initialized separately — see
/// [`crate::StandardMaterialRefPlugin`].
pub struct MaterialRefPlugin<L>
where
	L: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> L>,
}

impl<L> Default for MaterialRefPlugin<L>
where
	L: SystemParam + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<L> Plugin for MaterialRefPlugin<L>
where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(
				fulfill_material_ref_roots::<L>,
				fulfill_material_ref_descendants::<L>.after(fulfill_material_ref_roots::<L>),
			),
		);
	}
}
