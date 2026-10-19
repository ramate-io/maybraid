//! character [`MaterialLib`]: face, then clothing shader recipes.
//!
//! [`CharacterMaterialLib`] is the composable domain lib. Standalone apps
//! ([`CharacterMaterialRefPlugin`]) add [`StandardMaterial`] via
//! [`StandaloneCharacterMaterialLib`]. Maybraid World nests [`CharacterMaterialLib`]
//! and falls through to vegetation.

use bevy::ecs::system::SystemParam;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use character_items::{ClothingMaterial, FirearmMaterial};
use character_shaders::{ClothingShaderMaterial, FaceShaderKind, FaceShaderMaterial};
use material_ref::{
	material_ref_plugin_installed, MaterialId, MaterialLib, MaterialRef, MaterialRefCache,
	MaterialRefKey, MaterialRefPlugin, MaterialRefRoot, PropagateToDescendants,
	StandardMaterialLib, StandardMaterialRefCache,
};

use crate::eye_glow::EyeGlowMaterialInstance;

/// Cache of resolved [`FaceShaderMaterial`] handles (shared + per-character instance).
#[derive(Resource, Debug, Default)]
pub struct FaceShaderMaterialRefCache {
	shared: HashMap<MaterialRefKey, Handle<FaceShaderMaterial>>,
	instanced: HashMap<(MaterialRefKey, Entity), Handle<FaceShaderMaterial>>,
}

impl FaceShaderMaterialRefCache {
	pub fn len(&self) -> usize {
		self.shared.len() + self.instanced.len()
	}

	pub fn instanced_len(&self) -> usize {
		self.instanced.len()
	}

	/// Drops instanced handles whose [`EyeGlowMaterialInstance::owner`] no longer exists.
	pub fn prune_instanced_owners(
		&mut self,
		owner_alive: impl Fn(Entity) -> bool,
		materials: &mut Assets<FaceShaderMaterial>,
	) {
		self.instanced.retain(|(_key, owner), handle| {
			if owner_alive(*owner) {
				return true;
			}
			materials.remove(handle);
			false
		});
	}
}

/// Cache of resolved [`ClothingShaderMaterial`] handles.
pub type ClothingShaderMaterialRefCache = MaterialRefCache<ClothingShaderMaterial>;

/// Inserts clothing and face material caches. Idempotent.
pub fn init_character_material_caches(app: &mut App) {
	app.init_resource::<StandardMaterialRefCache>()
		.init_resource::<ClothingShaderMaterialRefCache>()
		.init_resource::<FaceShaderMaterialRefCache>();
}

/// Claims face recipe names only. Does not fall through to [`StandardMaterial`].
#[derive(SystemParam)]
pub struct FaceMaterialLib<'w, 's> {
	pub face_materials: ResMut<'w, Assets<FaceShaderMaterial>>,
	pub face_cache: ResMut<'w, FaceShaderMaterialRefCache>,
	parents: Query<'w, 's, &'static ChildOf>,
	propagating_roots: Query<'w, 's, &'static MaterialRefRoot, With<PropagateToDescendants>>,
	instances: Query<'w, 's, &'static EyeGlowMaterialInstance>,
}

impl FaceMaterialLib<'_, '_> {
	fn resolve_face(
		&mut self,
		material_ref: &MaterialRef,
		mesh_entity: Entity,
	) -> Handle<FaceShaderMaterial> {
		let key = MaterialRefKey::from(material_ref);
		let owner = face_material_owner(
			mesh_entity,
			&self.parents,
			&self.propagating_roots,
			&self.instances,
		);
		if let Some(owner) = owner {
			if let Some(handle) = self.face_cache.instanced.get(&(key.clone(), owner)) {
				return handle.clone();
			}
			let handle = self
				.face_materials
				.add(FaceShaderMaterial::from_material_ref(material_ref));
			self.face_cache.instanced.insert((key, owner), handle.clone());
			return handle;
		}
		if let Some(handle) = self.face_cache.shared.get(&key) {
			return handle.clone();
		}
		let handle = self.face_materials.add(FaceShaderMaterial::from_material_ref(material_ref));
		self.face_cache.shared.insert(key, handle.clone());
		handle
	}
}

fn face_material_owner(
	mesh_entity: Entity,
	parents: &Query<&ChildOf>,
	propagating_roots: &Query<&MaterialRefRoot, With<PropagateToDescendants>>,
	instances: &Query<&EyeGlowMaterialInstance>,
) -> Option<Entity> {
	let mut entity = mesh_entity;
	for _ in 0..64 {
		if propagating_roots.get(entity).is_ok() {
			return instances.get(entity).ok().map(|instance| instance.owner);
		}
		entity = parents.get(entity).ok()?.parent();
	}
	None
}

impl MaterialLib for FaceMaterialLib<'_, '_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		match &material_ref.name {
			MaterialId::Name(name) if FaceShaderKind::is_face_recipe(name) => {
				let handle = self.resolve_face(material_ref, entity);
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

/// Face + clothing / firearm recipes. Does not fall through to [`StandardMaterial`].
///
/// Standalone apps use [`StandaloneCharacterMaterialLib`]. Composed apps (Maybraid
/// World) nest this and supply their own fallback.
#[derive(SystemParam)]
pub struct CharacterMaterialLib<'w, 's> {
	pub face: FaceMaterialLib<'w, 's>,
	pub clothing: ClothingMaterialLib<'w>,
}

impl MaterialLib for CharacterMaterialLib<'_, '_> {
	fn try_fulfill(
		&mut self,
		entity: Entity,
		material_ref: &MaterialRef,
		commands: &mut Commands,
	) -> bool {
		self.face.try_fulfill(entity, material_ref, commands)
			|| self.clothing.try_fulfill(entity, material_ref, commands)
	}

	fn fulfill(&mut self, entity: Entity, material_ref: &MaterialRef, commands: &mut Commands) {
		let _ = self.try_fulfill(entity, material_ref, commands);
	}
}

/// Menu / playground fulfill: character, then green [`StandardMaterial`].
#[derive(SystemParam)]
pub struct StandaloneCharacterMaterialLib<'w, 's> {
	pub character: CharacterMaterialLib<'w, 's>,
	pub standard: StandardMaterialLib<'w>,
}

impl MaterialLib for StandaloneCharacterMaterialLib<'_, '_> {
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

/// Registers clothing / face caches + [`MaterialRefPlugin`] for
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
		app.add_plugins(MaterialRefPlugin::<StandaloneCharacterMaterialLib<'_, '_>>::default());
	}
}
