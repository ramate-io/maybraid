//! Composed [`MaterialLib`] for Maybraid World.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use building_shaders::{
	BuildingShadersPlugin, UrbanSurfaceMaterialLib, init_urban_material_caches,
};
use characters::CharacterShadersPlugin;
use characters::material_lib::{CharacterMaterialLib, init_character_material_caches};
use chico::VegetationOnTerrainMaterialLib;
use firearms::{
	EnergyMaterialLib, EnergyMaterialPlugin, MuzzleFlameMaterialLib, MuzzleFlameMaterialPlugin,
	init_energy_material_caches, init_muzzle_flame_caches,
};
use furniture_shaders::{
	FurnitureMaterialLib, FurnitureShadersPlugin, init_furniture_material_caches,
};
use material_ref::{MaterialLib, MaterialRef, MaterialRefPlugin};
use vegetation_bumpout::init_bump_out_material_caches;
use vegetation_shaders::{VegetationShadersPlugin, init_vegetation_material_caches};

/// World-model lib: furniture kits, character face / clothing, Richmond urban
/// surfaces, the muzzle flame, energy looks, then vegetation and Standard.
///
/// Furniture recipes (`furniture_wood`, …) must be claimed **before** urban
/// `wood`. Urban recipes (`stucco`, `wood`, …), `muzzle_flame`, and the
/// `laser_*` energy recipes must be claimed **before** vegetation's
/// [`StandardMaterial`] fallback.
///
/// Further domain libs (Durham recipes on [`MaterialRef`], sky) compose here.
#[derive(SystemParam)]
pub struct WorldMaterialLib<'w> {
	pub furniture: FurnitureMaterialLib<'w>,
	pub character: CharacterMaterialLib<'w>,
	pub urban: UrbanSurfaceMaterialLib<'w>,
	pub muzzle: MuzzleFlameMaterialLib<'w>,
	pub energy: EnergyMaterialLib<'w>,
	pub vegetation: VegetationOnTerrainMaterialLib<'w>,
}

impl MaterialLib for WorldMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.furniture.try_fulfill(entity, material_ref, commands)
			|| self.character.try_fulfill(entity, material_ref, commands)
			|| self.urban.try_fulfill(entity, material_ref, commands)
			|| self.muzzle.try_fulfill(entity, material_ref, commands)
			|| self.energy.try_fulfill(entity, material_ref, commands)
			|| self.vegetation.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Maybraid World composition-root plugin: domain caches + [`WorldMaterialLib`].
///
/// Domain material plugins initialize caches and shaders only. This plugin
/// installs the unique resolver. Isolated hosts (the materials playground)
/// also add [`WorldMaterialsShadersPlugin`].
///
/// Does not register domain [`MaterialPlugin`]s — layers still do that.
pub struct WorldMaterialRefPlugin;

impl Plugin for WorldMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_furniture_material_caches(app);
		init_character_material_caches(app);
		init_urban_material_caches(app);
		init_muzzle_flame_caches(app);
		init_energy_material_caches(app);
		init_vegetation_material_caches(app);
		init_bump_out_material_caches(app);
		app.add_plugins(MaterialRefPlugin::<WorldMaterialLib<'_>>::default());
	}
}

/// Domain shader plugins used by [`WorldMaterialLib`] recipes.
///
/// Idempotent: layer stacks that already registered a shader plugin skip.
pub struct WorldMaterialsShadersPlugin;

impl Plugin for WorldMaterialsShadersPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FurnitureShadersPlugin>() {
			app.add_plugins(FurnitureShadersPlugin);
		}
		if !app.is_plugin_added::<CharacterShadersPlugin>() {
			app.add_plugins(CharacterShadersPlugin);
		}
		if !app.is_plugin_added::<BuildingShadersPlugin>() {
			app.add_plugins(BuildingShadersPlugin);
		}
		if !app.is_plugin_added::<MuzzleFlameMaterialPlugin>() {
			app.add_plugins(MuzzleFlameMaterialPlugin);
		}
		if !app.is_plugin_added::<EnergyMaterialPlugin>() {
			app.add_plugins(EnergyMaterialPlugin);
		}
		if !app.is_plugin_added::<VegetationShadersPlugin>() {
			app.add_plugins(VegetationShadersPlugin);
		}
	}
}

/// Caches, fulfill, and domain shaders — enough for a standalone materials host.
pub struct WorldMaterialsPlugin;

impl Plugin for WorldMaterialsPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((WorldMaterialsShadersPlugin, WorldMaterialRefPlugin));
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::System;
	use bevy::prelude::*;
	use building_shaders::{
		ColumnSurfaceMaterial, ColumnSurfaceMaterialRefCache, RECIPE_DORIC_COLUMN, RECIPE_WOOD,
		UrbanMaterialRefPlugin, UrbanSurfaceMaterial, UrbanSurfaceMaterialRefCache,
	};
	use character_shaders::{ClothingShaderMaterial, FaceShaderMaterial, RECIPE_FACE_EYE};
	use characters::material_lib::{
		CharacterMaterialRefPlugin, ClothingShaderMaterialRefCache, FaceShaderMaterialRefCache,
	};
	use chico::VegetationOnTerrainMaterialRefPlugin;
	use firearms::{
		EnergyMaterial, EnergyMaterialRefCache, LASER_HEX_RECIPE, MUZZLE_FLAME_RECIPE,
		MuzzleFlameMaterial, MuzzleFlameMaterialRefCache,
	};
	use furniture_shaders::{
		FurnitureMaterialRefPlugin, FurnitureSurfaceMaterial, FurnitureSurfaceMaterialRefCache,
		RECIPE_FURNITURE_WOOD,
	};
	use material_ref::{
		MaterialRef, MaterialRefApplied, MaterialRefRoot, material_ref_plugin_installed,
	};
	use vegetation_bumpout::{BUMP_OUT_MATERIAL, BumpOutMaterial, BumpOutMaterialRefCache};
	use vegetation_shaders::{
		FrondMaterial, FrondMaterialRefCache, LeafMaterial, LeafMaterialRefCache, StickMaterial,
		StickMaterialRefCache, VegetationMaterialRefPlugin,
	};

	use crate::material_lib::WorldMaterialRefPlugin;

	fn world_fulfill_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<FurnitureSurfaceMaterial>()
			.init_asset::<FaceShaderMaterial>()
			.init_asset::<ClothingShaderMaterial>()
			.init_asset::<UrbanSurfaceMaterial>()
			.init_asset::<ColumnSurfaceMaterial>()
			.init_asset::<MuzzleFlameMaterial>()
			.init_asset::<EnergyMaterial>()
			.init_asset::<BumpOutMaterial>()
			.init_asset::<LeafMaterial>()
			.init_asset::<StickMaterial>()
			.init_asset::<FrondMaterial>()
			.add_plugins(WorldMaterialRefPlugin);
		app
	}

	fn spawn_named(app: &mut App, name: &str) -> Entity {
		app.world_mut().spawn(MaterialRefRoot(MaterialRef::named(name))).id()
	}

	#[test]
	fn world_material_plugin_initializes_character_material_caches() {
		let mut app = App::new();
		app.add_plugins(WorldMaterialRefPlugin);
		assert!(material_ref_plugin_installed(&app));
		assert!(app.world().contains_resource::<ClothingShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<FaceShaderMaterialRefCache>());
		assert!(app.world().contains_resource::<UrbanSurfaceMaterialRefCache>());
		assert!(app.world().contains_resource::<ColumnSurfaceMaterialRefCache>());
		assert!(app.world().contains_resource::<FurnitureSurfaceMaterialRefCache>());
		assert!(app.world().contains_resource::<MuzzleFlameMaterialRefCache>());
		assert!(app.world().contains_resource::<EnergyMaterialRefCache>());
		assert!(app.world().contains_resource::<LeafMaterialRefCache>());
		assert!(app.world().contains_resource::<StickMaterialRefCache>());
		assert!(app.world().contains_resource::<FrondMaterialRefCache>());
		assert!(app.world().contains_resource::<BumpOutMaterialRefCache>());
	}

	#[test]
	fn domain_cache_plugins_after_world_keep_the_world_resolver() {
		let mut app = App::new();
		app.add_plugins(WorldMaterialRefPlugin)
			.add_plugins(CharacterMaterialRefPlugin)
			.add_plugins(UrbanMaterialRefPlugin)
			.add_plugins(FurnitureMaterialRefPlugin)
			.add_plugins(VegetationMaterialRefPlugin)
			.add_plugins(VegetationOnTerrainMaterialRefPlugin);
		assert!(material_ref_plugin_installed(&app));
	}

	#[test]
	fn world_lib_preserves_composed_recipe_priority() {
		let mut app = world_fulfill_app();
		let furniture = spawn_named(&mut app, RECIPE_FURNITURE_WOOD);
		let face = spawn_named(&mut app, RECIPE_FACE_EYE);
		let urban = spawn_named(&mut app, RECIPE_WOOD);
		let column = spawn_named(&mut app, RECIPE_DORIC_COLUMN);
		let flame = spawn_named(&mut app, MUZZLE_FLAME_RECIPE);
		let energy = spawn_named(&mut app, LASER_HEX_RECIPE);
		let bump = spawn_named(&mut app, BUMP_OUT_MATERIAL);
		let leaf = spawn_named(&mut app, "CHICO_LEAF_MATERIAL");
		let fallback = spawn_named(&mut app, "tuft");
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(furniture).is_some());
		assert!(app.world().get::<MeshMaterial3d<FurnitureSurfaceMaterial>>(furniture).is_some());
		assert!(app.world().get::<MeshMaterial3d<FaceShaderMaterial>>(face).is_some());
		assert!(app.world().get::<MeshMaterial3d<UrbanSurfaceMaterial>>(urban).is_some());
		assert!(app.world().get::<MeshMaterial3d<ColumnSurfaceMaterial>>(column).is_some());
		assert!(app.world().get::<MeshMaterial3d<MuzzleFlameMaterial>>(flame).is_some());
		assert!(app.world().get::<MeshMaterial3d<EnergyMaterial>>(energy).is_some());
		assert!(app.world().get::<MeshMaterial3d<BumpOutMaterial>>(bump).is_some());
		assert!(app.world().get::<MeshMaterial3d<LeafMaterial>>(leaf).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(fallback).is_some());
		assert!(app.world().get::<MeshMaterial3d<UrbanSurfaceMaterial>>(furniture).is_none());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(urban).is_none());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(column).is_none());
	}

	fn deferred_material_completion<M: Material>(recipe: &str, retire: bool) -> Result<(), String> {
		let mut app = world_fulfill_app();
		app.set_error_handler(bevy::ecs::error::panic);
		let world = app.world_mut();
		let mut descendants = Vec::new();
		for _ in 0..2 {
			let root = world
				.spawn((
					MaterialRefRoot(MaterialRef::named(recipe)),
					material_ref::PropagateToDescendants,
				))
				.id();
			let mesh = world
				.spawn((
					ChildOf(root),
					Mesh3d(Handle::default()),
					MeshMaterial3d::<StandardMaterial>(Handle::default()),
				))
				.id();
			descendants.push((root, mesh));
		}
		let mut fulfill = IntoSystem::into_system(
			material_ref::fulfill_material_ref_descendants::<super::WorldMaterialLib<'_>>,
		);
		fulfill.initialize(world);
		fulfill
			.run_without_applying_deferred((), world)
			.map_err(|error| error.to_string())?;
		assert!(world.get::<MeshMaterial3d<M>>(descendants[0].1).is_none());
		if retire {
			// A streamed root can disappear after a new mesh is discovered but
			// before the material library's deferred commands are applied.
			assert!(world.despawn(descendants[0].0));
		}
		fulfill.apply_deferred(world);
		for (index, (root, mesh)) in descendants.into_iter().enumerate() {
			if retire && index == 0 {
				assert!(world.get_entity(root).is_err());
				assert!(world.get_entity(mesh).is_err());
			} else {
				let material =
					world.get::<MeshMaterial3d<M>>(mesh).ok_or("live material missing")?;
				assert!(world.resource::<Assets<M>>().contains(material.id()));
				assert!(world.get::<MeshMaterial3d<StandardMaterial>>(mesh).is_none());
				assert!(world.get::<MaterialRefApplied>(mesh).is_some());
			}
		}
		Ok(())
	}

	#[test]
	fn material_completion_preserves_live_custom_materials() -> Result<(), String> {
		deferred_material_completion::<FurnitureSurfaceMaterial>(RECIPE_FURNITURE_WOOD, false)?;
		deferred_material_completion::<UrbanSurfaceMaterial>(RECIPE_WOOD, false)?;
		deferred_material_completion::<ColumnSurfaceMaterial>(RECIPE_DORIC_COLUMN, false)?;
		deferred_material_completion::<LeafMaterial>("CHICO_LEAF_MATERIAL", false)?;
		deferred_material_completion::<StickMaterial>("CHICO_STICK_MATERIAL", false)?;
		deferred_material_completion::<FrondMaterial>("CHICO_FROND_MATERIAL", false)?;
		deferred_material_completion::<BumpOutMaterial>(BUMP_OUT_MATERIAL, false)?;
		deferred_material_completion::<MuzzleFlameMaterial>(MUZZLE_FLAME_RECIPE, false)?;
		deferred_material_completion::<EnergyMaterial>(LASER_HEX_RECIPE, false)?;
		Ok(())
	}

	#[test]
	fn material_completion_tolerates_retired_furniture_mesh() -> Result<(), String> {
		deferred_material_completion::<FurnitureSurfaceMaterial>(RECIPE_FURNITURE_WOOD, true)
	}
	#[test]
	fn material_completion_tolerates_retired_urban_mesh() -> Result<(), String> {
		deferred_material_completion::<UrbanSurfaceMaterial>(RECIPE_WOOD, true)
	}
	#[test]
	fn material_completion_tolerates_retired_column_mesh() -> Result<(), String> {
		deferred_material_completion::<ColumnSurfaceMaterial>(RECIPE_DORIC_COLUMN, true)
	}
	#[test]
	fn material_completion_tolerates_retired_leaf_mesh() -> Result<(), String> {
		deferred_material_completion::<LeafMaterial>("CHICO_LEAF_MATERIAL", true)
	}
	#[test]
	fn material_completion_tolerates_retired_stick_mesh() -> Result<(), String> {
		deferred_material_completion::<StickMaterial>("CHICO_STICK_MATERIAL", true)
	}
	#[test]
	fn material_completion_tolerates_retired_frond_mesh() -> Result<(), String> {
		deferred_material_completion::<FrondMaterial>("CHICO_FROND_MATERIAL", true)
	}
	#[test]
	fn material_completion_tolerates_retired_bump_out_mesh() -> Result<(), String> {
		deferred_material_completion::<BumpOutMaterial>(BUMP_OUT_MATERIAL, true)
	}
	#[test]
	fn material_completion_tolerates_retired_muzzle_mesh() -> Result<(), String> {
		deferred_material_completion::<MuzzleFlameMaterial>(MUZZLE_FLAME_RECIPE, true)
	}
	#[test]
	fn material_completion_tolerates_retired_energy_mesh() -> Result<(), String> {
		deferred_material_completion::<EnergyMaterial>(LASER_HEX_RECIPE, true)
	}
}
