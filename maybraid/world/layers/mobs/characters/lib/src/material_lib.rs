//! character [`MaterialLib`]: face, clothing, then skin-tattoo shader recipes.
//!
//! [`CharacterMaterialLib`] is the composable domain lib. Standalone apps
//! ([`CharacterMaterialRefPlugin`]) add [`StandardMaterial`] via
//! [`StandaloneCharacterMaterialLib`]. Maybraid World nests [`CharacterMaterialLib`]
//! and falls through to vegetation.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use character_items::{ClothingMaterial, FirearmMaterial};
use character_shaders::{
	ClothingShaderMaterial, FaceShaderKind, FaceShaderMaterial, SkinShaderMaterial, SkinTattooMask,
	SkinTattooMasks,
};
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, StandardMaterialLib, StandardMaterialRefCache,
};

/// Cache of resolved [`ClothingShaderMaterial`] handles.
pub type ClothingShaderMaterialRefCache = MaterialRefCache<ClothingShaderMaterial>;

/// Cache of resolved [`FaceShaderMaterial`] handles.
pub type FaceShaderMaterialRefCache = MaterialRefCache<FaceShaderMaterial>;

/// Cache of resolved [`SkinShaderMaterial`] handles.
pub type SkinShaderMaterialRefCache = MaterialRefCache<SkinShaderMaterial>;

/// Inserts clothing, face, and skin-tattoo material caches. Idempotent.
pub fn init_character_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<ClothingShaderMaterialRefCache>()
		.init_resource::<FaceShaderMaterialRefCache>()
		.init_resource::<SkinShaderMaterialRefCache>();
	if !app.world().contains_resource::<SkinTattooMasks>() {
		app.init_resource::<SkinTattooMasks>();
	}
}

/// Claims face recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct FaceMaterialLib<'w> {
	pub face_materials: ResMut<'w, Assets<FaceShaderMaterial>>,
	pub face_cache: ResMut<'w, FaceShaderMaterialRefCache>,
}

impl FaceMaterialLib<'_> {
	fn resolve_face(&mut self, material_ref: &MaterialRef) -> Handle<FaceShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.face_cache.get(&key) {
			return handle;
		}
		let handle = self.face_materials.add(FaceShaderMaterial::from_material_ref(material_ref));
		self.face_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for FaceMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if FaceShaderKind::is_face_recipe(name) => {
				let handle = self.resolve_face(material_ref);
				commands
					.entity(entity)
					.try_remove::<MeshMaterial3d<StandardMaterial>>()
					.try_insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Claims clothing recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct ClothingMaterialLib<'w> {
	pub clothing_materials: ResMut<'w, Assets<ClothingShaderMaterial>>,
	pub clothing_cache: ResMut<'w, ClothingShaderMaterialRefCache>,
}

impl ClothingMaterialLib<'_> {
	fn resolve_clothing(&mut self, material_ref: &MaterialRef) -> Handle<ClothingShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.clothing_cache.get(&key) {
			return handle;
		}
		let handle = self
			.clothing_materials
			.add(ClothingShaderMaterial::from_material_ref(material_ref));
		self.clothing_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for ClothingMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name)
				if ClothingMaterial::is_clothing_recipe(name)
					|| FirearmMaterial::is_firearm_recipe(name) =>
			{
				let handle = self.resolve_clothing(material_ref);
				commands
					.entity(entity)
					.try_remove::<MeshMaterial3d<StandardMaterial>>()
					.try_insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Claims skin-tattoo recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct SkinMaterialLib<'w> {
	pub skin_materials: ResMut<'w, Assets<SkinShaderMaterial>>,
	pub skin_cache: ResMut<'w, SkinShaderMaterialRefCache>,
	pub masks: Res<'w, SkinTattooMasks>,
}

impl SkinMaterialLib<'_> {
	fn resolve_skin(&mut self, material_ref: &MaterialRef) -> Handle<SkinShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		if let Some(handle) = self.skin_cache.get(&key) {
			return handle;
		}
		let mask = match &material_ref.name {
			MaterialId::Name(name) => self.masks.handle_for_recipe(name),
			MaterialId::Default => self.masks.body.clone(),
		};
		let handle = self
			.skin_materials
			.add(SkinShaderMaterial::from_material_ref(material_ref, mask));
		self.skin_cache.insert(key, handle.clone());
		handle
	}
}

impl MaterialLib for SkinMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if SkinTattooMask::is_skin_tattoo_recipe(name) => {
				let handle = self.resolve_skin(material_ref);
				commands
					.entity(entity)
					.try_remove::<MeshMaterial3d<StandardMaterial>>()
					.try_insert(MeshMaterial3d(handle));
				true
			}
			_ => false,
		}
	}
}

/// Face + clothing / firearm + skin-tattoo recipes. Does not fall through to [`StandardMaterial`].
///
/// Standalone apps use [`StandaloneCharacterMaterialLib`]. Composed apps (Maybraid
/// World) nest this and supply their own fallback.
#[derive(SystemParam)]
pub struct CharacterMaterialLib<'w> {
	pub face: FaceMaterialLib<'w>,
	pub clothing: ClothingMaterialLib<'w>,
	pub skin: SkinMaterialLib<'w>,
}

impl MaterialLib for CharacterMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.face.try_fulfill(entity, material_ref, commands)
			|| self.clothing.try_fulfill(entity, material_ref, commands)
			|| self.skin.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Menu / playground fulfill: character, then green [`StandardMaterial`].
#[derive(SystemParam)]
pub struct StandaloneCharacterMaterialLib<'w> {
	pub character: CharacterMaterialLib<'w>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for StandaloneCharacterMaterialLib<'_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.character.try_fulfill(entity, material_ref, commands)
			|| self.standard.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Registers clothing / face / skin-tattoo caches + [`MaterialRefPlugin`] for
/// [`StandaloneCharacterMaterialLib`].
///
/// Composed apps that already installed a fulfill plugin (Maybraid World) skip
/// here; they nest [`CharacterMaterialLib`] instead.
pub struct CharacterMaterialRefPlugin;

impl Plugin for CharacterMaterialRefPlugin {
	fn build(&self, app: &mut App) {
		init_character_material_caches(app);
		if material_ref_plugin_installed(app) {
			return;
		}
		app.add_plugins(MaterialRefPlugin::<StandaloneCharacterMaterialLib<'_>>::default());
	}
}
