//! Playground [`MaterialLib`]: leaf / stick / frond named recipes + green default.

use bevy::ecs::system::SystemParam;
use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use chico_vegetation_components::{
	CHICO_FROND_MATERIAL, CHICO_LEAF_MATERIAL, CHICO_STICK_MATERIAL,
};
use chico_vegetation_shaders::{ChicoFrondMaterial, ChicoLeafMaterial, ChicoStickMaterial};
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};

/// Cache of resolved [`ChicoLeafMaterial`] handles.
pub type ChicoLeafMaterialRefCache = MaterialRefCache<ChicoLeafMaterial>;

/// Cache of resolved [`ChicoStickMaterial`] handles.
pub type ChicoStickMaterialRefCache = MaterialRefCache<ChicoStickMaterial>;

/// Cache of resolved [`ChicoFrondMaterial`] handles.
pub type ChicoFrondMaterialRefCache = MaterialRefCache<ChicoFrondMaterial>;

/// Multi-type lib: leaf / stick / frond named recipes + green [`StandardMaterial`] default.
#[derive(SystemParam)]
pub struct ChicoMaterialLib<'w> {
	pub standard: StandardMaterialLib<'w>,
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
	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		match &material_ref.name {
			MaterialId::Name(name) if name == CHICO_LEAF_MATERIAL => {
				let handle = self.resolve_leaf(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
			}
			MaterialId::Name(name) if name == CHICO_STICK_MATERIAL => {
				let handle = self.resolve_stick(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
			}
			MaterialId::Name(name) if name == CHICO_FROND_MATERIAL => {
				let handle = self.resolve_frond(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle))
					.insert(NotShadowCaster);
			}
			_ => {
				self.standard.fulfill(entity, material_ref, commands);
				commands.entity(entity).insert(NotShadowCaster);
			}
		}
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

/// Registers caches + [`MaterialRefPlugin`] for [`ChicoMaterialLib`].
pub struct ChicoMaterialRefPlugin;

impl Plugin for ChicoMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<StandardMaterialRefCache>()
			.init_resource::<ChicoLeafMaterialRefCache>()
			.init_resource::<ChicoStickMaterialRefCache>()
			.init_resource::<ChicoFrondMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<ChicoMaterialLib<'_>>::default());
	}
}
