use std::collections::HashMap;

use bevy::prelude::*;

use crate::{preview::PreviewAssetTarget, preview_color::PreviewColor, skinning::CharacterPart, thumbnail::ThumbnailPreview};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedPreviewColor(PreviewColor);

#[derive(Resource, Default)]
pub struct PreviewColorMaterials {
	handles: HashMap<PreviewColor, Handle<StandardMaterial>>,
}

impl PreviewColorMaterials {
	fn handle(
		&mut self,
		color: PreviewColor,
		materials: &mut Assets<StandardMaterial>,
	) -> Handle<StandardMaterial> {
		self.handles
			.entry(color)
			.or_insert_with(|| {
				materials.add(StandardMaterial {
					base_color: color.bevy_color(),
					cull_mode: None,
					..default()
				})
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
	mut meshes: Query<(
		Entity,
		&mut MeshMaterial3d<StandardMaterial>,
		Option<&AppliedPreviewColor>,
	)>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut color_materials: ResMut<PreviewColorMaterials>,
) {
	for (root, target, children) in &preview_roots {
		apply_color_to_tree(
			root,
			target.color,
			children,
			&character_parts,
			&children_q,
			&mut meshes,
			&mut materials,
			&mut color_materials,
			&mut commands,
		);
	}
	for (root, preview, children) in &thumbnail_roots {
		apply_color_to_tree(
			root,
			preview.color,
			children,
			&character_parts,
			&children_q,
			&mut meshes,
			&mut materials,
			&mut color_materials,
			&mut commands,
		);
	}
}

fn apply_color_to_tree(
	root: Entity,
	color: PreviewColor,
	children: Option<&Children>,
	character_parts: &Query<Entity, With<CharacterPart>>,
	children_q: &Query<&Children>,
	meshes: &mut Query<(
		Entity,
		&mut MeshMaterial3d<StandardMaterial>,
		Option<&AppliedPreviewColor>,
	)>,
	materials: &mut Assets<StandardMaterial>,
	color_materials: &mut PreviewColorMaterials,
	commands: &mut Commands,
) {
	let Some(children) = children else {
		return;
	};
	let handle = color_materials.handle(color, materials);
	let mut stack: Vec<(Entity, bool)> = children.iter().map(|entity| (entity, false)).collect();
	while let Some((entity, is_entry)) = stack.pop() {
		if !is_entry && character_parts.contains(entity) && entity != root {
			continue;
		}
		if let Ok((mesh, mut material, applied)) = meshes.get_mut(entity) {
			if applied.is_some_and(|applied| applied.0 == color) && material.0 == handle {
				continue;
			}
			material.0 = handle.clone();
			commands.entity(mesh).try_insert(AppliedPreviewColor(color));
		}
		if let Ok(children) = children_q.get(entity) {
			for child in children.iter() {
				stack.push((child, false));
			}
		}
	}
}
