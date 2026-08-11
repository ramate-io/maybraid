//! Fulfill [`MaterialRefRoot`] via a generic [`MaterialLib`] [`SystemParam`].

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::{App, Commands, Entity, Plugin, Query, Update, Without};

use crate::lib_trait::MaterialLib;
use crate::material_ref::{MaterialRefApplied, MaterialRefRoot};

/// For each [`MaterialRefRoot`] without [`MaterialRefApplied`], ask `L` to insert a material.
pub fn fulfill_material_ref_roots<L>(
	mut commands: Commands,
	query: Query<(Entity, &MaterialRefRoot), Without<MaterialRefApplied>>,
	lib: StaticSystemParam<L>,
) where
	L: SystemParam + 'static,
	for<'w, 's> L::Item<'w, 's>: MaterialLib,
{
	let mut lib = lib.into_inner();
	for (entity, root) in &query {
		lib.fulfill(entity, &root.0, &mut commands);
		commands.entity(entity).insert(MaterialRefApplied);
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
		app.add_systems(Update, fulfill_material_ref_roots::<L>);
	}
}
