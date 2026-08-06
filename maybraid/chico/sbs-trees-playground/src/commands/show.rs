//! `/show` — LodScene presentation (VegetationComponents).

use bevy::prelude::*;
use chico_sbs_trees::SopesBanyan;
use chico_vegetation_components::{spawn_vegetation_components, vegetation_bounds};
use clap::{Args, Subcommand};

use crate::render::SbsRenderItem;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Sope's Banyan via VegetationComponents / LodScene.
	SopesBanyan(ShowSopesBanyan),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowSopesBanyan {
	#[command(flatten)]
	pub tree: SopesBanyan,
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		match self {
			Self::SopesBanyan(args) => {
				commands.insert_resource(ShowConfig {
					subject: Some(ShowSubject::SopesBanyan(args.tree)),
				});
			}
		}
	}
}

#[derive(Resource, Default)]
pub struct ShowConfig {
	pub subject: Option<ShowSubject>,
}

#[derive(Clone, Debug)]
pub enum ShowSubject {
	SopesBanyan(SopesBanyan),
}

#[derive(Component)]
pub struct ShowRoot;

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
		ShowSubject::SopesBanyan(params) => {
			let tree = params.build();
			let bounds = vegetation_bounds(&tree);
			let entities =
				spawn_vegetation_components(&mut commands, &tree, Transform::IDENTITY, bounds);
			for entity in entities {
				commands.entity(entity).insert(ShowRoot);
			}
		}
	}
}
