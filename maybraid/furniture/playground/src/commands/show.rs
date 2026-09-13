//! `/show` catalog: unit assemblies and the Richmond slot gallery.

pub mod transform;

use bevy::prelude::*;
use clap::{Args, Subcommand};
use richmond_building_components::FurnitureGeometry;

use crate::preview::{PreviewConfig, PreviewSubject};

pub use transform::ShowTransform;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Unit bed slot (frame / mattress / covers).
	Bed(UnitShow),
	/// Unit chair slot (legs / seat / +Z back).
	Chair(UnitShow),
	/// Unit chest slot (trunk / lid).
	Chest(UnitShow),
	/// Unit counter slot (footer / volume / top).
	Counter(UnitShow),
	/// Richmond bedroom / kitchen / living packers plus authored extremes.
	Gallery(GalleryShow),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct UnitShow {
	/// Palette key. Topology stays fixed.
	#[arg(long, default_value_t = 1)]
	pub seed: u64,
	#[command(flatten)]
	pub transform: ShowTransform,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct GalleryShow {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let (subject, transform) = match self {
			Self::Bed(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Bed, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Chair(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Chair, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Chest(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Chest, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Counter(cmd) => (
				PreviewSubject::Unit { geometry: FurnitureGeometry::Counter, seed: cmd.seed },
				cmd.transform.transform(),
			),
			Self::Gallery(cmd) => (PreviewSubject::Gallery, cmd.transform.transform()),
		};
		commands.insert_resource(PreviewConfig { subject, transform });
	}
}
