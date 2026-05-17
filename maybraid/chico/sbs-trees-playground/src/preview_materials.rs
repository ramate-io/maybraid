//! Preview vegetation [`Material`] handles (embedded WGSL from `chico-vegetation-shaders`).

use bevy::prelude::*;

use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};

use crate::preview::PreviewConfig;

/// Stable bark / foliage materials reused whenever [`PreviewConfig::tree`] is rebuilt from CLI defaults.
#[derive(Resource, Clone)]
pub struct PreviewTreeMaterials {
	pub stick: Handle<ChicoStickMaterial>,
	pub leaf: Handle<ChicoLeafMaterial>,
}

fn preview_stick_colors() -> ChicoStickMaterial {
	ChicoStickMaterial { base_color: Vec4::new(0.13, 0.085, 0.055, 1.0) }
}

fn preview_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.22, 0.5, 0.29, 1.0) }
}

pub fn setup_preview_tree_materials(
	mut commands: Commands,
	mut stick_assets: ResMut<Assets<ChicoStickMaterial>>,
	mut leaf_assets: ResMut<Assets<ChicoLeafMaterial>>,
	mut config: ResMut<PreviewConfig>,
) {
	let stick = stick_assets.add(preview_stick_colors());
	let leaf = leaf_assets.add(preview_leaf_colors());

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
