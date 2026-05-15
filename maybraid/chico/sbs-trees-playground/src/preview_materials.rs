//! Preview-only [`StandardMaterial`] handles for Sope's Banyan bark vs canopy reads.

use bevy::prelude::*;

use crate::preview::PreviewConfig;

/// Stable bark / foliage materials reused whenever [`PreviewConfig::tree`] is rebuilt from CLI defaults.
#[derive(Resource, Clone)]
pub struct PreviewTreeMaterials {
	pub stick: Handle<StandardMaterial>,
	pub leaf: Handle<StandardMaterial>,
}

fn dark_banyan_wood() -> StandardMaterial {
	StandardMaterial {
		base_color: Color::srgb(0.13, 0.085, 0.055),
		perceptual_roughness: 0.88,
		metallic: 0.03,
		..default()
	}
}

fn mid_canopy_green() -> StandardMaterial {
	StandardMaterial {
		base_color: Color::srgb(0.22, 0.5, 0.29),
		perceptual_roughness: 0.52,
		metallic: 0.0,
		..default()
	}
}

pub fn setup_preview_tree_materials(
	mut commands: Commands,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut config: ResMut<PreviewConfig>,
) {
	let stick = materials.add(dark_banyan_wood());
	let leaf = materials.add(mid_canopy_green());

	commands.insert_resource(PreviewTreeMaterials { stick: stick.clone(), leaf: leaf.clone() });

	config.tree.stick_material.mesh = MeshMaterial3d(stick);
	config.tree.leaf_material.mesh = MeshMaterial3d(leaf);
}

/// CLI parses [`SkippedMeshMaterial`] defaults as empty handles; reattach curated preview materials before spawning.
pub fn sync_preview_tree_material_handles(
	mut config: ResMut<PreviewConfig>,
	mats: Res<PreviewTreeMaterials>,
) {
	let stick = MeshMaterial3d(mats.stick.clone());
	let leaf = MeshMaterial3d(mats.leaf.clone());
	if config.tree.stick_material.mesh != stick || config.tree.leaf_material.mesh != leaf {
		config.tree.stick_material.mesh = stick;
		config.tree.leaf_material.mesh = leaf;
	}
}
