//! Preview vegetation [`Material`] handles (embedded WGSL from `chico-vegetation-shaders`).

use bevy::prelude::*;

use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};

use crate::preview::{PreviewConfig, PreviewTree};

/// Stable bark / foliage materials reused whenever [`PreviewConfig::tree`] is rebuilt from CLI defaults.
#[derive(Resource, Clone)]
pub struct PreviewTreeMaterials {
	pub stick: Handle<ChicoStickMaterial>,
	pub conifer_stick: Handle<ChicoStickMaterial>,
	pub leaf: Handle<ChicoLeafMaterial>,
	pub tuft: Handle<StandardMaterial>,
}

fn preview_stick_colors() -> ChicoStickMaterial {
	ChicoStickMaterial { base_color: Vec4::new(0.13, 0.085, 0.055, 1.0) }
}

fn preview_leaf_colors() -> ChicoLeafMaterial {
	ChicoLeafMaterial { base_color: Vec4::new(0.22, 0.5, 0.29, 1.0) }
}

fn preview_conifer_stick_colors() -> ChicoStickMaterial {
	ChicoStickMaterial { base_color: Vec4::new(0.18, 0.14, 0.10, 1.0) }
}

fn preview_tuft_standard_material() -> StandardMaterial {
	StandardMaterial {
		base_color: Color::srgb(0.22, 0.62, 0.28),
		double_sided: true,
		..Default::default()
	}
}

pub fn setup_preview_tree_materials(
	mut commands: Commands,
	mut stick_assets: ResMut<Assets<ChicoStickMaterial>>,
	mut leaf_assets: ResMut<Assets<ChicoLeafMaterial>>,
	mut standard_assets: ResMut<Assets<StandardMaterial>>,
	mut config: ResMut<PreviewConfig>,
) {
	let stick = stick_assets.add(preview_stick_colors());
	let conifer_stick = stick_assets.add(preview_conifer_stick_colors());
	let leaf = leaf_assets.add(preview_leaf_colors());
	let tuft = standard_assets.add(preview_tuft_standard_material());

	commands.insert_resource(PreviewTreeMaterials {
		stick: stick.clone(),
		conifer_stick: conifer_stick.clone(),
		leaf: leaf.clone(),
		tuft: tuft.clone(),
	});

	match &mut config.tree {
		PreviewTree::SopesBanyan(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(stick);
			tree.leaf_material.mesh = MeshMaterial3d(leaf);
		}
		PreviewTree::LiamsConifer(tree) => {
			tree.stick_material.mesh = MeshMaterial3d(conifer_stick);
			tree.leaf_material.mesh = MeshMaterial3d(tuft);
		}
	}
}

/// CLI parses [`SkippedMeshMaterial`] defaults as empty handles; reattach curated preview materials before spawning.
pub fn sync_preview_tree_material_handles(
	mut config: ResMut<PreviewConfig>,
	mats: Res<PreviewTreeMaterials>,
) {
	let stick = MeshMaterial3d(mats.stick.clone());
	let leaf = MeshMaterial3d(mats.leaf.clone());
	let tuft = MeshMaterial3d(mats.tuft.clone());

	match &mut config.tree {
		PreviewTree::SopesBanyan(tree) => {
			if tree.stick_material.mesh != stick || tree.leaf_material.mesh != leaf {
				tree.stick_material.mesh = stick;
				tree.leaf_material.mesh = leaf;
			}
		}
		PreviewTree::LiamsConifer(tree) => {
			let conifer = MeshMaterial3d(mats.conifer_stick.clone());
			if tree.stick_material.mesh != conifer || tree.leaf_material.mesh != tuft {
				tree.stick_material.mesh = conifer;
				tree.leaf_material.mesh = tuft;
			}
		}
	}
}
