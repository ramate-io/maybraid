//! [`MaterialLib`] for Richmond urban-surface shaders.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
};

use crate::{is_urban_surface_recipe, UrbanSurfaceMaterial};

/// Cache of resolved [`UrbanSurfaceMaterial`] handles.
pub type UrbanSurfaceMaterialRefCache = MaterialRefCache<UrbanSurfaceMaterial>;

/// Inserts urban-surface material caches. Idempotent.
pub fn init_richmond_urban_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<UrbanSurfaceMaterialRefCache>();
}

/// Claims urban recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct UrbanSurfaceMaterialLib<'w> {
	pub urban_materials: ResMut<'w, Assets<UrbanSurfaceMaterial>>,
	pub urban_cache: ResMut<'w, UrbanSurfaceMaterialRefCache>,
}

impl UrbanSurfaceMaterialLib<'_> {
	fn resolve(&mut self, material_ref: &MaterialRef) -> Handle<UrbanSurfaceMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.urban_cache.get(&key) {
			return handle;
		}
		let handle =
			self.urban_materials.add(UrbanSurfaceMaterial::from_material_ref(material_ref));
		self.urban_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for UrbanSurfaceMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if is_urban_surface_recipe(name) => {
				let handle = self.resolve(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Urban recipes, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct RichmondUrbanMaterialLib<'w> {
	pub urban: UrbanSurfaceMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for RichmondUrbanMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.urban.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers caches + [`MaterialRefPlugin`] for [`RichmondUrbanMaterialLib`].
pub struct RichmondUrbanMaterialRefPlugin;

impl Plugin for RichmondUrbanMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_richmond_urban_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<RichmondUrbanMaterialLib<'_>>::default());
	}
}
