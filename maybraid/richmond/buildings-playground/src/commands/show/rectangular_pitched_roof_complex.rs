//! `/show rectangular-pitched-roof-complex` — orthogonal AABB pitched roofs with valleys.

use bevy::prelude::*;
use clap::{Args, ValueEnum};
use richmond_buildings::{EndCap, Overhang, RectangularPitchedRoofComplexParams};

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum RoofComplexPreset {
	Single,
	#[default]
	L,
	T,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum EndCapKind {
	#[default]
	Hip,
	Gable,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RectangularPitchedRoofComplex {
	#[arg(long, value_enum, default_value_t = RoofComplexPreset::L)]
	pub preset: RoofComplexPreset,
	/// Side eave overhang in meters (fixed). Ignored when `--overhang-ratio` is set.
	#[arg(long, default_value_t = 0.3)]
	pub overhang_fixed: f32,
	/// Side eave overhang as a fraction of eave-to-eave span.
	#[arg(long)]
	pub overhang_ratio: Option<f32>,
	#[arg(long, value_enum, default_value_t = EndCapKind::Hip)]
	pub end_cap: EndCapKind,
	/// Gable ridge projection (meters) when `--end-cap gable`.
	#[arg(long, default_value_t = 0.4)]
	pub gable_ridge: f32,
	/// Gable eave projection (meters) when `--end-cap gable`.
	#[arg(long, default_value_t = 0.35)]
	pub gable_eave: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl RectangularPitchedRoofComplex {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::RectangularPitchedRoofComplex {
				preset: match self.preset {
					RoofComplexPreset::Single => "single".into(),
					RoofComplexPreset::L => "l".into(),
					RoofComplexPreset::T => "t".into(),
				},
				overhang_fixed: self.overhang_fixed,
				overhang_ratio: self.overhang_ratio,
				end_cap_gable: matches!(self.end_cap, EndCapKind::Gable),
				gable_ridge: self.gable_ridge,
				gable_eave: self.gable_eave,
			},
			self.transform.transform(),
		)
	}
}

pub fn build_params(
	preset: &str,
	overhang_fixed: f32,
	overhang_ratio: Option<f32>,
	end_cap_gable: bool,
	gable_ridge: f32,
	gable_eave: f32,
) -> RectangularPitchedRoofComplexParams {
	let mut params = match preset {
		"single" => RectangularPitchedRoofComplexParams::single(10.0, 6.0, 2.5, 4.5),
		"t" => RectangularPitchedRoofComplexParams::t_shape(),
		_ => RectangularPitchedRoofComplexParams::l_shape(),
	};
	params.overhang = if let Some(r) = overhang_ratio {
		Overhang::Ratio(r)
	} else {
		Overhang::Fixed(overhang_fixed)
	};
	params.end_cap = if end_cap_gable {
		EndCap::Gable {
			ridge: Overhang::Fixed(gable_ridge),
			eave: Overhang::Fixed(gable_eave),
		}
	} else {
		EndCap::Hip
	};
	params
}
