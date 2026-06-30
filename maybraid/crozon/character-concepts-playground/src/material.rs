use bevy::prelude::*;
use crozon_characters::species::braidman::BraidmanColor;

use crate::{preview::PreviewAssetTarget, thumbnail::ThumbnailPreview};

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AppliedPreviewColor(BraidmanColor);

pub fn apply_preview_colors(
	mut commands: Commands,
	preview_roots: Query<(&PreviewAssetTarget, Option<&Children>)>,
	thumbnail_roots: Query<(&ThumbnailPreview, Option<&Children>)>,
	children_q: Query<&Children>,
	mut meshes: Query<(
		Entity,
		&mut MeshMaterial3d<StandardMaterial>,
		Option<&AppliedPreviewColor>,
	)>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	for (target, children) in &preview_roots {
		apply_color_to_tree(
			target.color,
			children,
			&children_q,
			&mut meshes,
			&mut materials,
			&mut commands,
		);
	}
	for (preview, children) in &thumbnail_roots {
		apply_color_to_tree(
			preview.color,
			children,
			&children_q,
			&mut meshes,
			&mut materials,
			&mut commands,
		);
	}
}

fn apply_color_to_tree(
	color: BraidmanColor,
	children: Option<&Children>,
	children_q: &Query<&Children>,
	meshes: &mut Query<(
		Entity,
		&mut MeshMaterial3d<StandardMaterial>,
		Option<&AppliedPreviewColor>,
	)>,
	materials: &mut Assets<StandardMaterial>,
	commands: &mut Commands,
) {
	let Some(children) = children else {
		return;
	};
	let mut stack: Vec<Entity> = children.iter().collect();
	while let Some(entity) = stack.pop() {
		if let Ok((mesh, mut material, applied)) = meshes.get_mut(entity) {
			if applied.is_some_and(|applied| applied.0 == color) {
				continue;
			}
			let mut next = materials.get(&material.0).cloned().unwrap_or_default();
			next.base_color = color.color();
			material.0 = materials.add(next);
			commands.entity(mesh).insert(AppliedPreviewColor(color));
		}
		if let Ok(children) = children_q.get(entity) {
			stack.extend(children.iter());
		}
	}
}
