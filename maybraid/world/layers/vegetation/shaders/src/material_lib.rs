//! [`MaterialLib`] for vegetation shaders: leaf / stick / frond recipes only.

use bevy::ecs::system::SystemParam;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, StandardMaterialLib,
	StandardMaterialRefCache,
};
use vegetation_components::lod_host::StickGeometryRoot;
use vegetation_components::{CanopyShadowMode, FROND_MATERIAL, LEAF_MATERIAL, STICK_MATERIAL};

use crate::{FrondMaterial, LeafMaterial, StickMaterial};

/// Cache of resolved [`LeafMaterial`] handles.
pub type LeafMaterialRefCache = MaterialRefCache<LeafMaterial>;

/// Cache of resolved [`StickMaterial`] handles.
pub type StickMaterialRefCache = MaterialRefCache<StickMaterial>;

/// Cache of resolved [`FrondMaterial`] handles.
pub type FrondMaterialRefCache = MaterialRefCache<FrondMaterial>;

/// Inserts leaf / stick / frond material caches. Idempotent.
pub fn init_vegetation_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<LeafMaterialRefCache>()
		.init_resource::<StickMaterialRefCache>()
		.init_resource::<FrondMaterialRefCache>();
}

/// Leaf / stick / frond named recipes. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct VegetationMaterialLib<'w, 's> {
	pub leaf_materials: ResMut<'w, Assets<LeafMaterial>>,
	pub leaf_cache: ResMut<'w, LeafMaterialRefCache>,
	pub stick_materials: ResMut<'w, Assets<StickMaterial>>,
	pub stick_cache: ResMut<'w, StickMaterialRefCache>,
	pub frond_materials: ResMut<'w, Assets<FrondMaterial>>,
	pub frond_cache: ResMut<'w, FrondMaterialRefCache>,
	pub canopy_shadows: Option<Res<'w, CanopyShadowMode>>,
	pub parents: Query<'w, 's, &'static ChildOf>,
	pub sticks: Query<'w, 's, (), With<StickGeometryRoot>>,
	pub excluded: Query<'w, 's, (), With<NotShadowCaster>>,
}

impl VegetationMaterialLib<'_, '_> {
	fn resolve_leaf(&mut self, material_ref: &MaterialRef) -> Handle<LeafMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.leaf_cache.get(&key) {
			return handle;
		}
		let handle = self.leaf_materials.add(leaf_from_ref(material_ref));
		self.leaf_cache.insert(key, handle.clone());
		handle
	}

	fn resolve_stick(&mut self, material_ref: &MaterialRef) -> Handle<StickMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.stick_cache.get(&key) {
			return handle;
		}
		let handle = self.stick_materials.add(stick_from_ref(material_ref));
		self.stick_cache.insert(key, handle.clone());
		handle
	}

	fn resolve_frond(&mut self, material_ref: &MaterialRef) -> Handle<FrondMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.frond_cache.get(&key) {
			return handle;
		}
		let handle = self.frond_materials.add(frond_from_ref(material_ref));
		self.frond_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for VegetationMaterialLib<'_, '_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		let canopy_casts = self.canopy_shadows.as_ref().is_some_and(|mode| mode.canopy_casts());
		match &material_ref.name {
			MaterialId::Name(name) if name == LEAF_MATERIAL => {
				let handle = self.resolve_leaf(material_ref);
				fulfill_canopy_material(commands, entity, handle, canopy_casts);
				true
			}
			MaterialId::Name(name) if name == STICK_MATERIAL => {
				let handle = self.resolve_stick(material_ref);
				let fitted =
					self.canopy_shadows.as_ref().is_some_and(|m| **m == CanopyShadowMode::Fitted);
				let actual_stick = fitted && {
					let mut actual_stick = false;
					let mut independent_exclusion = false;
					let mut reached_root = false;
					let mut cursor = entity;
					for _ in 0..32 {
						actual_stick |= self.sticks.contains(cursor);
						independent_exclusion |= self.excluded.contains(cursor);
						let Ok(parent) = self.parents.get(cursor) else {
							reached_root = true;
							break;
						};
						cursor = parent.parent();
					}
					actual_stick && !independent_exclusion && reached_root
				};
				commands
					.entity(entity)
					.try_remove::<MeshMaterial3d<StandardMaterial>>()
					.try_insert(MeshMaterial3d(handle));
				if !actual_stick {
					commands.entity(entity).try_insert(NotShadowCaster);
				}
				true
			}
			MaterialId::Name(name) if name == FROND_MATERIAL => {
				let handle = self.resolve_frond(material_ref);
				fulfill_canopy_material(commands, entity, handle, canopy_casts);
				true
			}
			_ => false,
		}
	}
}

/// Shaders-crate standalone lib: Chico recipes, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct StandaloneVegetationMaterialLib<'w, 's> {
	pub chico: VegetationMaterialLib<'w, 's>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for StandaloneVegetationMaterialLib<'_, '_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.chico.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

fn fulfill_canopy_material<M: Asset + Clone + Material>(
	commands: &mut Commands,
	entity: Entity,
	handle: Handle<M>,
	canopy_casts: bool,
) {
	if canopy_casts {
		commands
			.entity(entity)
			.try_remove::<MeshMaterial3d<StandardMaterial>>()
			.try_insert(MeshMaterial3d(handle));
	} else {
		commands
			.entity(entity)
			.try_remove::<MeshMaterial3d<StandardMaterial>>()
			.try_insert(MeshMaterial3d(handle))
			.try_insert(NotShadowCaster);
	}
}

fn leaf_from_ref(material_ref: &MaterialRef) -> LeafMaterial {
	let mut mat = LeafMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

fn stick_from_ref(material_ref: &MaterialRef) -> StickMaterial {
	let mut mat = StickMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

fn frond_from_ref(material_ref: &MaterialRef) -> FrondMaterial {
	let mut mat = FrondMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

/// Idempotent leaf / stick / frond cache initializer. Does not install a resolver.
///
/// Isolated vegetation hosts install [`MaterialRefPlugin`]`<`[`StandaloneVegetationMaterialLib`]`>`
/// at their composition root. Composed apps nest [`VegetationMaterialLib`].
pub struct VegetationMaterialRefPlugin;

impl Plugin for VegetationMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_vegetation_material_caches(app);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use material_ref::{
		material_ref_plugin_installed, MaterialRef, MaterialRefApplied, MaterialRefPlugin,
		MaterialRefRoot, StandardMaterialRefCache,
	};
	use vegetation_components::LEAF_MATERIAL;

	use super::*;
	use crate::{FrondMaterial, LeafMaterial, StickMaterial};

	#[test]
	fn vegetation_material_ref_plugin_is_cache_only() {
		let mut app = App::new();
		app.add_plugins(VegetationMaterialRefPlugin);
		assert!(app.world().contains_resource::<LeafMaterialRefCache>());
		assert!(!material_ref_plugin_installed(&app));
	}

	#[test]
	fn standalone_vegetation_lib_fulfills_leaf_then_standard() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()))
			.init_asset::<StandardMaterial>()
			.init_asset::<LeafMaterial>()
			.init_asset::<StickMaterial>()
			.init_asset::<FrondMaterial>()
			.init_resource::<StandardMaterialRefCache>()
			.init_resource::<LeafMaterialRefCache>()
			.init_resource::<StickMaterialRefCache>()
			.init_resource::<FrondMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<StandaloneVegetationMaterialLib<'_, '_>>::default());

		let leaf = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named(LEAF_MATERIAL))).id();
		let other = app.world_mut().spawn(MaterialRefRoot(MaterialRef::named("tuft"))).id();
		app.update();

		assert!(app.world().get::<MaterialRefApplied>(leaf).is_some());
		assert!(app.world().get::<MeshMaterial3d<LeafMaterial>>(leaf).is_some());
		assert!(app.world().get::<MeshMaterial3d<StandardMaterial>>(other).is_some());
	}
	#[test]
	fn fitted_casts_actual_sticks_only_and_honors_ancestor_opt_out() {
		for mode in [CanopyShadowMode::Proxy, CanopyShadowMode::Fitted] {
			let mut app = App::new();
			app.add_plugins((MinimalPlugins, AssetPlugin::default()))
				.init_asset::<StandardMaterial>()
				.init_asset::<LeafMaterial>()
				.init_asset::<StickMaterial>()
				.init_asset::<FrondMaterial>()
				.init_resource::<StandardMaterialRefCache>()
				.init_resource::<LeafMaterialRefCache>()
				.init_resource::<StickMaterialRefCache>()
				.init_resource::<FrondMaterialRefCache>()
				.insert_resource(mode)
				.add_plugins(MaterialRefPlugin::<StandaloneVegetationMaterialLib<'_, '_>>::default())
				.add_systems(
					PostUpdate,
					vegetation_components::lod_host::inherit_not_shadow_caster_on_meshes,
				);
			let typed = app.world_mut().spawn(StickGeometryRoot).id();
			let excluded = app.world_mut().spawn((StickGeometryRoot, NotShadowCaster)).id();
			let actual = app
				.world_mut()
				.spawn((
					ChildOf(typed),
					Mesh3d::default(),
					MaterialRefRoot(MaterialRef::named(STICK_MATERIAL)),
				))
				.id();
			let opt_out = app
				.world_mut()
				.spawn((
					ChildOf(excluded),
					Mesh3d::default(),
					MaterialRefRoot(MaterialRef::named(STICK_MATERIAL)),
				))
				.id();
			let bark_column = app
				.world_mut()
				.spawn((Mesh3d::default(), MaterialRefRoot(MaterialRef::named(STICK_MATERIAL))))
				.id();
			let leaf =
				app.world_mut().spawn(MaterialRefRoot(MaterialRef::named(LEAF_MATERIAL))).id();
			app.update();
			app.update();
			assert_eq!(
				app.world().get::<NotShadowCaster>(actual).is_none(),
				mode == CanopyShadowMode::Fitted
			);
			for entity in [opt_out, bark_column, leaf] {
				assert!(app.world().get::<NotShadowCaster>(entity).is_some());
			}
		}
	}
}
