use std::collections::HashMap;

use bevy::prelude::*;
use exploratory_shaders::{EyeballAlbedo, EyeballShader, SplatterAlbedo, SplatterShader};

use crate::{
	preview::PreviewAssetTarget, preview_color::PreviewColor, skinning::CharacterPart,
	thumbnail::{self, ThumbnailPreview},
};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedPreviewColor(PreviewColor);

#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct AppliedThumbnailColor(Color);

#[derive(Resource, Default)]
pub struct PreviewColorMaterials {
	handles: HashMap<PreviewColor, Handle<SplatterShader>>,
	eye_handle: Option<Handle<EyeballShader>>,
	thumbnail_handles: HashMap<[u8; 4], Handle<SplatterShader>>,
	eye_thumbnail_handle: Option<Handle<EyeballShader>>,
}

impl PreviewColorMaterials {
	fn splatter_handle(
		&mut self,
		color: PreviewColor,
		materials: &mut Assets<SplatterShader>,
		albedo: &Handle<Image>,
	) -> Handle<SplatterShader> {
		self.handles
			.entry(color)
			.or_insert_with(|| {
				materials.add(SplatterShader::new(albedo.clone()).with_base_color(color.bevy_color()))
			})
			.clone()
	}

	fn eye_handle(
		&mut self,
		materials: &mut Assets<EyeballShader>,
		albedo: &Handle<Image>,
	) -> Handle<EyeballShader> {
		if let Some(handle) = &self.eye_handle {
			return handle.clone();
		}
		let handle = materials.add(EyeballShader::new(albedo.clone()));
		self.eye_handle = Some(handle.clone());
		handle
	}

	fn splatter_thumbnail_handle(
		&mut self,
		color: Color,
		materials: &mut Assets<SplatterShader>,
		albedo: &Handle<Image>,
	) -> Handle<SplatterShader> {
		let key = color_key(color);
		self.thumbnail_handles
			.entry(key)
			.or_insert_with(|| {
				materials.add(SplatterShader::new(albedo.clone()).with_base_color(color))
			})
			.clone()
	}

	fn eye_thumbnail_handle(
		&mut self,
		materials: &mut Assets<EyeballShader>,
		albedo: &Handle<Image>,
	) -> Handle<EyeballShader> {
		if let Some(handle) = &self.eye_thumbnail_handle {
			return handle.clone();
		}
		let handle = materials.add(EyeballShader::new(albedo.clone()));
		self.eye_thumbnail_handle = Some(handle.clone());
		handle
	}
}

pub fn apply_preview_colors(
	mut commands: Commands,
	preview_roots: Query<(Entity, &PreviewAssetTarget, Option<&Children>)>,
	thumbnail_roots: Query<(Entity, &ThumbnailPreview, Option<&Children>)>,
	character_parts: Query<Entity, With<CharacterPart>>,
	children_q: Query<&Children>,
	standard_meshes: Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: Query<(Entity, &MeshMaterial3d<SplatterShader>)>,
	eyeball_meshes: Query<(Entity, &MeshMaterial3d<EyeballShader>)>,
	splatter_albedo: Res<SplatterAlbedo>,
	eyeball_albedo: Res<EyeballAlbedo>,
	mut splatter_materials: ResMut<Assets<SplatterShader>>,
	mut eyeball_materials: ResMut<Assets<EyeballShader>>,
	mut color_materials: ResMut<PreviewColorMaterials>,
) {
	for (root, target, children) in &preview_roots {
		if target.target.is_eye() {
			let handle =
				color_materials.eye_handle(&mut eyeball_materials, &eyeball_albedo.0);
			apply_eye_material_to_tree(
				root,
				children,
				&character_parts,
				&children_q,
				&standard_meshes,
				&splatter_meshes,
				&eyeball_meshes,
				&handle,
				&mut commands,
			);
		} else {
			let handle = color_materials.splatter_handle(
				target.color,
				&mut splatter_materials,
				&splatter_albedo.0,
			);
			apply_splatter_material_to_tree(
				root,
				children,
				&character_parts,
				&children_q,
				&standard_meshes,
				&splatter_meshes,
				&eyeball_meshes,
				&handle,
				&mut commands,
				|commands, entity| {
					commands.entity(entity).try_insert(AppliedPreviewColor(target.color));
				},
			);
		}
	}
	for (root, preview, children) in &thumbnail_roots {
		if thumbnail::is_eye_asset_path(preview.asset_path) {
			let handle =
				color_materials.eye_thumbnail_handle(&mut eyeball_materials, &eyeball_albedo.0);
			apply_eye_material_to_tree(
				root,
				children,
				&character_parts,
				&children_q,
				&standard_meshes,
				&splatter_meshes,
				&eyeball_meshes,
				&handle,
				&mut commands,
			);
		} else {
			let handle = color_materials.splatter_thumbnail_handle(
				preview.color,
				&mut splatter_materials,
				&splatter_albedo.0,
			);
			apply_splatter_material_to_tree(
				root,
				children,
				&character_parts,
				&children_q,
				&standard_meshes,
				&splatter_meshes,
				&eyeball_meshes,
				&handle,
				&mut commands,
				|commands, entity| {
					commands.entity(entity).try_insert(AppliedThumbnailColor(preview.color));
				},
			);
		}
	}
}

fn apply_splatter_material_to_tree(
	root: Entity,
	children: Option<&Children>,
	character_parts: &Query<Entity, With<CharacterPart>>,
	children_q: &Query<&Children>,
	standard_meshes: &Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: &Query<(Entity, &MeshMaterial3d<SplatterShader>)>,
	eyeball_meshes: &Query<(Entity, &MeshMaterial3d<EyeballShader>)>,
	handle: &Handle<SplatterShader>,
	commands: &mut Commands,
	mark_applied: impl Fn(&mut Commands, Entity),
) {
	let Some(children) = children else {
		return;
	};
	let mut stack: Vec<Entity> = children.iter().collect();
	while let Some(entity) = stack.pop() {
		if character_parts.contains(entity) && entity != root {
			continue;
		}
		try_apply_splatter_material(
			entity,
			handle,
			standard_meshes,
			splatter_meshes,
			eyeball_meshes,
			commands,
			&mark_applied,
		);
		if let Ok(children) = children_q.get(entity) {
			stack.extend(children.iter());
		}
	}
}

fn apply_eye_material_to_tree(
	root: Entity,
	children: Option<&Children>,
	character_parts: &Query<Entity, With<CharacterPart>>,
	children_q: &Query<&Children>,
	standard_meshes: &Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: &Query<(Entity, &MeshMaterial3d<SplatterShader>)>,
	eyeball_meshes: &Query<(Entity, &MeshMaterial3d<EyeballShader>)>,
	handle: &Handle<EyeballShader>,
	commands: &mut Commands,
) {
	let Some(children) = children else {
		return;
	};
	let mut stack: Vec<Entity> = children.iter().collect();
	while let Some(entity) = stack.pop() {
		if character_parts.contains(entity) && entity != root {
			continue;
		}
		try_apply_eyeball_material(
			entity,
			handle,
			standard_meshes,
			splatter_meshes,
			eyeball_meshes,
			commands,
		);
		if let Ok(children) = children_q.get(entity) {
			stack.extend(children.iter());
		}
	}
}

fn try_apply_splatter_material(
	entity: Entity,
	handle: &Handle<SplatterShader>,
	standard_meshes: &Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: &Query<(Entity, &MeshMaterial3d<SplatterShader>)>,
	eyeball_meshes: &Query<(Entity, &MeshMaterial3d<EyeballShader>)>,
	commands: &mut Commands,
	mark_applied: &impl Fn(&mut Commands, Entity),
) {
	if standard_meshes.contains(entity) || eyeball_meshes.get(entity).is_ok() {
		commands
			.entity(entity)
			.remove::<MeshMaterial3d<StandardMaterial>>()
			.remove::<MeshMaterial3d<EyeballShader>>()
			.insert(MeshMaterial3d(handle.clone()));
		mark_applied(commands, entity);
		return;
	}

	if let Ok((_, material)) = splatter_meshes.get(entity) {
		if material.0 == *handle {
			return;
		}
		commands.entity(entity).insert(MeshMaterial3d(handle.clone()));
		mark_applied(commands, entity);
	}
}

fn try_apply_eyeball_material(
	entity: Entity,
	handle: &Handle<EyeballShader>,
	standard_meshes: &Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: &Query<(Entity, &MeshMaterial3d<SplatterShader>)>,
	eyeball_meshes: &Query<(Entity, &MeshMaterial3d<EyeballShader>)>,
	commands: &mut Commands,
) {
	if standard_meshes.contains(entity) || splatter_meshes.get(entity).is_ok() {
		commands
			.entity(entity)
			.remove::<MeshMaterial3d<StandardMaterial>>()
			.remove::<MeshMaterial3d<SplatterShader>>()
			.insert(MeshMaterial3d(handle.clone()));
		return;
	}

	if let Ok((_, material)) = eyeball_meshes.get(entity) {
		if material.0 == *handle {
			return;
		}
		commands.entity(entity).insert(MeshMaterial3d(handle.clone()));
	}
}

fn color_key(color: Color) -> [u8; 4] {
	let color = color.to_srgba();
	[
		(color.red * 255.0).round() as u8,
		(color.green * 255.0).round() as u8,
		(color.blue * 255.0).round() as u8,
		(color.alpha * 255.0).round() as u8,
	]
}
