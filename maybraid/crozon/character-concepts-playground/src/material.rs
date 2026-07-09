use std::collections::HashMap;

use bevy::prelude::*;
use exploratory_shaders::{SplatterAlbedo, SplatterShader};

use crate::{
	preview::PreviewAssetTarget, preview_color::PreviewColor, skinning::CharacterPart,
	thumbnail::ThumbnailPreview,
};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedPreviewColor(PreviewColor);

#[derive(Component, Clone, Copy, PartialEq)]
pub(crate) struct AppliedThumbnailColor(Color);

#[derive(Resource, Default)]
pub struct PreviewColorMaterials {
	handles: HashMap<PreviewColor, Handle<SplatterShader>>,
	thumbnail_handles: HashMap<[u8; 4], Handle<SplatterShader>>,
}

impl PreviewColorMaterials {
	fn handle(
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

	fn thumbnail_handle(
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
}

pub fn apply_preview_colors(
	mut commands: Commands,
	preview_roots: Query<(Entity, &PreviewAssetTarget, Option<&Children>)>,
	thumbnail_roots: Query<(Entity, &ThumbnailPreview, Option<&Children>)>,
	character_parts: Query<Entity, With<CharacterPart>>,
	children_q: Query<&Children>,
	standard_meshes: Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: Query<(
		Entity,
		&MeshMaterial3d<SplatterShader>,
		Option<&AppliedPreviewColor>,
	)>,
	splatter_albedo: Res<SplatterAlbedo>,
	mut materials: ResMut<Assets<SplatterShader>>,
	mut color_materials: ResMut<PreviewColorMaterials>,
) {
	let albedo = &splatter_albedo.0;
	for (root, target, children) in &preview_roots {
		apply_preview_color_to_tree(
			root,
			target.color,
			children,
			&character_parts,
			&children_q,
			&standard_meshes,
			&splatter_meshes,
			&mut materials,
			&mut color_materials,
			albedo,
			&mut commands,
		);
	}
	for (root, preview, children) in &thumbnail_roots {
		apply_thumbnail_color_to_tree(
			root,
			preview.color,
			children,
			&character_parts,
			&children_q,
			&standard_meshes,
			&splatter_meshes,
			&mut materials,
			&mut color_materials,
			albedo,
			&mut commands,
		);
	}
}

fn apply_preview_color_to_tree(
	root: Entity,
	color: PreviewColor,
	children: Option<&Children>,
	character_parts: &Query<Entity, With<CharacterPart>>,
	children_q: &Query<&Children>,
	standard_meshes: &Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: &Query<(
		Entity,
		&MeshMaterial3d<SplatterShader>,
		Option<&AppliedPreviewColor>,
	)>,
	materials: &mut Assets<SplatterShader>,
	color_materials: &mut PreviewColorMaterials,
	albedo: &Handle<Image>,
	commands: &mut Commands,
) {
	let Some(children) = children else {
		return;
	};
	let handle = color_materials.handle(color, materials, albedo);
	let mut stack: Vec<Entity> = children.iter().collect();
	while let Some(entity) = stack.pop() {
		if character_parts.contains(entity) && entity != root {
			continue;
		}
		try_apply_splatter_material(
			entity,
			&handle,
			standard_meshes,
			splatter_meshes,
			commands,
			|commands, entity| {
				commands.entity(entity).try_insert(AppliedPreviewColor(color));
			},
			|applied| applied.is_some_and(|applied| applied.0 == color),
		);
		if let Ok(children) = children_q.get(entity) {
			stack.extend(children.iter());
		}
	}
}

fn apply_thumbnail_color_to_tree(
	root: Entity,
	color: Color,
	children: Option<&Children>,
	character_parts: &Query<Entity, With<CharacterPart>>,
	children_q: &Query<&Children>,
	standard_meshes: &Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: &Query<(
		Entity,
		&MeshMaterial3d<SplatterShader>,
		Option<&AppliedPreviewColor>,
	)>,
	materials: &mut Assets<SplatterShader>,
	color_materials: &mut PreviewColorMaterials,
	albedo: &Handle<Image>,
	commands: &mut Commands,
) {
	let Some(children) = children else {
		return;
	};
	let handle = color_materials.thumbnail_handle(color, materials, albedo);
	let mut stack: Vec<Entity> = children.iter().collect();
	while let Some(entity) = stack.pop() {
		if character_parts.contains(entity) && entity != root {
			continue;
		}
		try_apply_splatter_material(
			entity,
			&handle,
			standard_meshes,
			splatter_meshes,
			commands,
			|commands, entity| {
				commands.entity(entity).try_insert(AppliedThumbnailColor(color));
			},
			|_| false,
		);
		if let Ok(children) = children_q.get(entity) {
			stack.extend(children.iter());
		}
	}
}

/// Swaps GLTF [`StandardMaterial`] meshes to [`SplatterShader`], or updates an existing handle.
fn try_apply_splatter_material(
	entity: Entity,
	handle: &Handle<SplatterShader>,
	standard_meshes: &Query<Entity, With<MeshMaterial3d<StandardMaterial>>>,
	splatter_meshes: &Query<(
		Entity,
		&MeshMaterial3d<SplatterShader>,
		Option<&AppliedPreviewColor>,
	)>,
	commands: &mut Commands,
	mark_applied: impl FnOnce(&mut Commands, Entity),
	already_applied: impl FnOnce(Option<&AppliedPreviewColor>) -> bool,
) -> bool {
	if standard_meshes.contains(entity) {
		commands
			.entity(entity)
			.remove::<MeshMaterial3d<StandardMaterial>>()
			.insert(MeshMaterial3d(handle.clone()));
		mark_applied(commands, entity);
		return true;
	}

	if let Ok((_, material, applied)) = splatter_meshes.get(entity) {
		if already_applied(applied) && material.0 == *handle {
			return true;
		}
		commands.entity(entity).insert(MeshMaterial3d(handle.clone()));
		mark_applied(commands, entity);
		return true;
	}

	false
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
