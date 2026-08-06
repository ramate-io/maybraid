//! `/show` — LodScene presentation (VegetationComponents).

use bevy::prelude::*;
use chico_sbs_trees::{
	KamakuraTorchParams, PenmarchTorchParams, RorysHeadTrainedParams, SopesBanyanParams,
};
use chico_vegetation_components::{
	spawn_vegetation_components, vegetation_bounds, VegetationComponents,
};
use clap::{Args, Subcommand};

use crate::render::SbsRenderItem;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Sope's Banyan via VegetationComponents / LodScene.
	SopesBanyan(ShowSopesBanyan),
	/// Penmarch Torch via VegetationComponents / LodScene.
	PenmarchTorch(ShowPenmarchTorch),
	/// Kamakura Torch via VegetationComponents / LodScene.
	KamakuraTorch(ShowKamakuraTorch),
	/// Rory's Head-trained via VegetationComponents / LodScene.
	RorysHeadTrained(ShowRorysHeadTrained),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowSopesBanyan {
	#[command(flatten)]
	pub tree: SopesBanyanParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowPenmarchTorch {
	#[command(flatten)]
	pub tree: PenmarchTorchParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowKamakuraTorch {
	#[command(flatten)]
	pub tree: KamakuraTorchParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowRorysHeadTrained {
	#[command(flatten)]
	pub tree: RorysHeadTrainedParams,
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let subject = match self {
			Self::SopesBanyan(args) => ShowSubject::SopesBanyan(args.tree),
			Self::PenmarchTorch(args) => ShowSubject::PenmarchTorch(args.tree),
			Self::KamakuraTorch(args) => ShowSubject::KamakuraTorch(args.tree),
			Self::RorysHeadTrained(args) => ShowSubject::RorysHeadTrained(args.tree),
		};
		commands.insert_resource(ShowConfig { subject: Some(subject) });
	}
}

#[derive(Resource, Default)]
pub struct ShowConfig {
	pub subject: Option<ShowSubject>,
}

#[derive(Clone, Debug)]
pub enum ShowSubject {
	SopesBanyan(SopesBanyanParams),
	PenmarchTorch(PenmarchTorchParams),
	KamakuraTorch(KamakuraTorchParams),
	RorysHeadTrained(RorysHeadTrainedParams),
}

#[derive(Component)]
pub struct ShowRoot;

fn spawn_show_tree(
	commands: &mut Commands,
	tree: &impl VegetationComponents,
) {
	let bounds = vegetation_bounds(tree);
	let entities = spawn_vegetation_components(commands, tree, Transform::IDENTITY, bounds);
	for entity in entities {
		commands.entity(entity).insert(ShowRoot);
	}
}

/// Present `/show` subjects when `ShowConfig` changes. Clears legacy `/render` roots.
pub fn sync_show(
	mut commands: Commands,
	config: Res<ShowConfig>,
	show_roots: Query<Entity, With<ShowRoot>>,
	render_roots: Query<Entity, (With<SbsRenderItem>, Without<ChildOf>)>,
	mut last: Local<Option<String>>,
) {
	let key = match &config.subject {
		None => None,
		Some(ShowSubject::SopesBanyan(t)) => Some(format!("sopes-banyan:{:?}", t.geometry)),
		Some(ShowSubject::PenmarchTorch(t)) => Some(format!("penmarch-torch:{:?}", t.geometry)),
		Some(ShowSubject::KamakuraTorch(t)) => Some(format!("kamakura-torch:{:?}", t.geometry)),
		Some(ShowSubject::RorysHeadTrained(t)) => {
			Some(format!("rorys-head-trained:{:?}", t.geometry))
		}
	};
	if key == *last && show_roots.iter().next().is_some() {
		return;
	}
	for entity in &show_roots {
		commands.entity(entity).despawn();
	}
	*last = key.clone();
	let Some(subject) = &config.subject else {
		return;
	};

	for entity in &render_roots {
		commands.entity(entity).despawn();
	}

	match subject {
		ShowSubject::SopesBanyan(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::PenmarchTorch(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::KamakuraTorch(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::RorysHeadTrained(params) => spawn_show_tree(&mut commands, &params.build()),
	}
}
