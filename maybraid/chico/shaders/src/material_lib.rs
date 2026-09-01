//! [`MaterialLib`] for Chico vegetation shaders: leaf / stick / frond recipes only.

use bevy::ecs::system::SystemParam;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use chico_vegetation_components::{
	CHICO_FROND_MATERIAL, CHICO_LEAF_MATERIAL, CHICO_STICK_MATERIAL,
};
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};

use crate::{ChicoFrondMaterial, ChicoLeafMaterial, ChicoStickMaterial};

/// Cache of resolved [`ChicoLeafMaterial`] handles.
pub type ChicoLeafMaterialRefCache = MaterialRefCache<ChicoLeafMaterial>;

/// Cache of resolved [`ChicoStickMaterial`] handles.
pub type ChicoStickMaterialRefCache = MaterialRefCache<ChicoStickMaterial>;

/// Cache of resolved [`ChicoFrondMaterial`] handles.
pub type ChicoFrondMaterialRefCache = MaterialRefCache<ChicoFrondMaterial>;

/// Inserts leaf / stick / frond material caches. Idempotent.
pub fn init_chico_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<ChicoLeafMaterialRefCache>()
		.init_resource::<ChicoStickMaterialRefCache>()
		.init_resource::<ChicoFrondMaterialRefCache>();
}

/// Leaf / stick / frond named recipes. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct ChicoMaterialLib<'w> {
	pub leaf_materials: ResMut<'w, Assets<ChicoLeafMaterial>>,
	pub leaf_cache: ResMut<'w, ChicoLeafMaterialRefCache>,
	pub stick_materials: ResMut<'w, Assets<ChicoStickMaterial>>,
	pub stick_cache: ResMut<'w, ChicoStickMaterialRefCache>,
	pub frond_materials: ResMut<'w, Assets<ChicoFrondMaterial>>,
	pub frond_cache: ResMut<'w, ChicoFrondMaterialRefCache>,
}

impl ChicoMaterialLib<'_> {
	fn resolve_leaf(&mut self, material_ref: &MaterialRef) -> Handle<ChicoLeafMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.leaf_cache.get(&key) {
			return handle;
		}
		let handle = self.leaf_materials.add(leaf_from_ref(material_ref));
		self.leaf_cache.insert(key, handle.clone());
		handle
	}

	fn resolve_stick(&mut self, material_ref: &MaterialRef) -> Handle<ChicoStickMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.stick_cache.get(&key) {
			return handle;
		}
		let handle = self.stick_materials.add(stick_from_ref(material_ref));
		self.stick_cache.insert(key, handle.clone());
		handle
	}

	fn resolve_frond(&mut self, material_ref: &MaterialRef) -> Handle<ChicoFrondMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.frond_cache.get(&key) {
			return handle;
		}
		let handle = self.frond_materials.add(frond_from_ref(material_ref));
		self.frond_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for ChicoMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if name == CHICO_LEAF_MATERIAL => {
				let handle = self.resolve_leaf(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
				true
			}
			MaterialId::Name(name) if name == CHICO_STICK_MATERIAL => {
				let handle = self.resolve_stick(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
				true
			}
			MaterialId::Name(name) if name == CHICO_FROND_MATERIAL => {
				let handle = self.resolve_frond(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
				true
			}
			_ => false,
		}
	}
}

/// Shaders-crate standalone lib: Chico recipes, then [`StandardMaterialLib`].
#[derive(SystemParam)]
pub struct ChicoStandaloneMaterialLib<'w> {
	pub chico: ChicoMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for ChicoStandaloneMaterialLib<'_> {
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

fn leaf_from_ref(material_ref: &MaterialRef) -> ChicoLeafMaterial {
	let mut mat = ChicoLeafMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

fn stick_from_ref(material_ref: &MaterialRef) -> ChicoStickMaterial {
	let mut mat = ChicoStickMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

fn frond_from_ref(material_ref: &MaterialRef) -> ChicoFrondMaterial {
	let mut mat = ChicoFrondMaterial::default();
	if let Some(color) = material_ref.palette.first() {
		let linear = LinearRgba::from(*color);
		mat.base_color = Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	mat
}

/// Registers caches + [`MaterialRefPlugin`] for [`ChicoStandaloneMaterialLib`].
///
/// Vegetation / world apps that compose several domain libs should call
/// [`init_chico_material_caches`] and skip this plugin.
pub struct ChicoMaterialRefPlugin;

impl Plugin for ChicoMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_chico_material_caches(app);
		if material_ref::material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<ChicoStandaloneMaterialLib<'_>>::default());
	}
}
