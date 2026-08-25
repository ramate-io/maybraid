//! `/show triangular-panels` — isolated right-triangle kits + live LOD print.

use bevy::prelude::*;
use clap::{Args, ValueEnum};
use richmond_building_components::panels::PanelStyle;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum TrianglePanelStyle {
	#[default]
	RoughStonework,
	Flat,
	ShepherdsThatch,
	Default,
	DesertWeb,
	MyrsOrnate,
	RibAndPlank,
	TentAngles,
	TerracottaTubes,
}

impl TrianglePanelStyle {
	pub fn panel_style(self) -> PanelStyle {
		match self {
			Self::RoughStonework => PanelStyle::RoughStonework,
			Self::Flat => PanelStyle::Flat,
			Self::ShepherdsThatch => PanelStyle::ShepherdsThatch,
			Self::Default => PanelStyle::Default,
			Self::DesertWeb => PanelStyle::DesertWeb,
			Self::MyrsOrnate => PanelStyle::MyrsOrnate,
			Self::RibAndPlank => PanelStyle::RibAndPlank,
			Self::TentAngles => PanelStyle::TentAngles,
			Self::TerracottaTubes => PanelStyle::TerracottaTubes,
		}
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TriangularPanels {
	/// Panel kit style (Low = style triad; UltraLow = shared flat).
	#[arg(long, value_enum, default_value_t = TrianglePanelStyle::RoughStonework)]
	pub style: TrianglePanelStyle,
	/// Plan size of each unit kit in meters (`--scale` is the shared x,y,z transform).
	#[arg(long, default_value_t = 1.0)]
	pub size: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl TriangularPanels {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::TriangularPanels {
				style: self.style.panel_style(),
				scale: self.size.max(1e-3),
			},
			self.transform.transform(),
		)
	}
}
