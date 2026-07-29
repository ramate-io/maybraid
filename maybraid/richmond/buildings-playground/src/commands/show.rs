//! `/show` subcommand: preview a partition leaf or authored building.

pub mod arc_180;
pub mod arc_90;
pub mod bedroom;
pub mod header_90;
pub mod linear;
pub mod linear_wall;
pub mod polyline;
pub mod polyline_wall;
pub mod stacked_rings;
pub mod transform;
pub mod wizards_tower;

use bevy::prelude::*;
use clap::Subcommand;

pub use transform::ShowTransform;

use crate::preview::PreviewConfig;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Straight rough-stonework linear segment (`rough_stonework_001.glb`).
	Linear(linear::Linear),
	/// 90° rough-stonework arc (`rough_stonework_90_001.glb`).
	Arc90(arc_90::Arc90),
	/// 180° rough-stonework arc (`rough_stonework_180_001.glb`).
	Arc180(arc_180::Arc180),
	/// 90° header rough-stonework (`rough_stonework_90_header_001.glb`).
	Header90(header_90::Header90),
	/// L-shaped `Partition::polyline` (posed linears + empty joint kit).
	Polyline(polyline::Polyline),
	/// Portal-sensitive straight [`richmond_buildings::LinearWall`].
	LinearWall(linear_wall::LinearWall),
	/// Portal-sensitive [`richmond_buildings::PolylineWall`] (L-path + door).
	PolylineWall(polyline_wall::PolylineWall),
	/// Full Wizard's Tower (noise-derived floor count).
	WizardsTower(wizards_tower::WizardsTower),
	/// Stacked circular wall rings (validates kit radius/height scaling).
	StackedRings(stacked_rings::StackedRings),
	/// Hierarchical bedroom (closet / bed / nightstand / ensuite placeholders).
	Bedroom(bedroom::Bedroom),
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let (subject, transform) = match self {
			Self::Linear(cmd) => cmd.into_preview(),
			Self::Arc90(cmd) => cmd.into_preview(),
			Self::Arc180(cmd) => cmd.into_preview(),
			Self::Header90(cmd) => cmd.into_preview(),
			Self::Polyline(cmd) => cmd.into_preview(),
			Self::LinearWall(cmd) => cmd.into_preview(),
			Self::PolylineWall(cmd) => cmd.into_preview(),
			Self::WizardsTower(cmd) => cmd.into_preview(),
			Self::StackedRings(cmd) => cmd.into_preview(),
			Self::Bedroom(cmd) => cmd.into_preview(),
		};
		commands.insert_resource(PreviewConfig { subject, transform });
	}
}
