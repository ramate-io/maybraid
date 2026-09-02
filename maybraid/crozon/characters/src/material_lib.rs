//! Crozon [`MaterialLib`]: clothing shader recipes + [`StandardMaterial`] fallthrough.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use crozon_character_items::ClothingMaterial;
use crozon_character_shaders::{ClothingShaderKind, ClothingShaderMaterial};
use material_ref::{
	MaterialId, MaterialLib, MaterialRef, MaterialRefCache, MaterialRefKey, MaterialRefPlugin,
	StandardMaterialLib, StandardMaterialRefCache,
};

/// Cache of resolved [`ClothingShaderMaterial`] handles.
pub type ClothingShaderMaterialRefCache = MaterialRefCache<ClothingShaderMaterial>;

/// Multi-type lib: named clothing recipes + green [`StandardMaterial`] default.
#[derive(SystemParam)]
pub struct CrozonMaterialLib<'w> {
	pub standard: StandardMaterialLib<'w>,
	pub clothing_materials: ResMut<'w, Assets<ClothingShaderMaterial>>,
	pub clothing_cache: ResMut<'w, ClothingShaderMaterialRefCache>,
}

impl CrozonMaterialLib<'_> {
	fn resolve_clothing(&mut self, material_ref: &MaterialRef) -> Handle<ClothingShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.clothing_cache.get(&key) {
			return handle;
		}
		let handle = self.clothing_materials.add(clothing_from_ref(material_ref));
		self.clothing_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for CrozonMaterialLib<'_> {
	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		match &material_ref.name {
			MaterialId::Name(name) if ClothingMaterial::is_clothing_recipe(name) => {
				let handle = self.resolve_clothing(material_ref);
				commands
					.entity(entity)
					.remove::<MeshMaterial3d<StandardMaterial>>()
					.insert(MeshMaterial3d(handle));
			}
			_ => self.standard.fulfill(entity, material_ref, commands),
		}
	}
}

fn clothing_from_ref(material_ref: &MaterialRef) -> ClothingShaderMaterial {
	let kind = match &material_ref.name {
		MaterialId::Name(name) if name == ClothingMaterial::SpaceSuit.recipe_id() => {
			ClothingShaderKind::SpaceSuit
		}
		MaterialId::Name(name) if name == ClothingMaterial::Tattered.recipe_id() => {
			ClothingShaderKind::Tattered
		}
		MaterialId::Name(name) if name == ClothingMaterial::Hawaiian.recipe_id() => {
			ClothingShaderKind::Hawaiian
		}
		MaterialId::Name(name) if name == ClothingMaterial::Scales.recipe_id() => {
			ClothingShaderKind::Scales
		}
		MaterialId::Name(name) if name == ClothingMaterial::WizardsVeins.recipe_id() => {
			ClothingShaderKind::WizardsVeins
		}
		MaterialId::Name(name) if name == ClothingMaterial::Glitter.recipe_id() => {
			ClothingShaderKind::Glitter
		}
		_ => ClothingShaderKind::Cloth,
	};
	let base_color = material_ref
		.palette
		.first()
		.map(|color| {
			let linear = LinearRgba::from(*color);
			Vec4::new(linear.red, linear.green, linear.blue, linear.alpha)
		})
		.unwrap_or(Vec4::new(0.46, 0.60, 0.72, 1.0));
	ClothingShaderMaterial::new(kind, base_color)
}

/// Registers clothing caches + [`MaterialRefPlugin`] for [`CrozonMaterialLib`].
pub struct CrozonMaterialRefPlugin;

impl Plugin for CrozonMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<StandardMaterialRefCache>()
			.init_resource::<ClothingShaderMaterialRefCache>()
			.add_plugins(MaterialRefPlugin::<CrozonMaterialLib<'_>>::default());
	}
}
