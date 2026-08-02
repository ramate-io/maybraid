//! `/show pitched-rectangular-roof` — two-half pitched roof (rectangular hip by default).

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PitchedRectangularRoof {
	/// Length along the ridge / eaves (X).
	#[arg(long, default_value_t = 10.0)]
	pub footprint_x: f32,
	/// Full span between the two eaves (Z).
	#[arg(long, default_value_t = 6.0)]
	pub footprint_z: f32,
	#[arg(long, default_value_t = 4.0)]
	pub ridge_height: f32,
	#[arg(long, default_value_t = 2.5)]
	pub eave_height: f32,
	/// How far each ridge end sits in from the eave ends.
	#[arg(long, default_value_t = 1.5)]
	pub ridge_inset: f32,
	/// Also draw half-gable end walling under the hips.
	#[arg(long, default_value_t = false)]
	pub gables: bool,
	/// Omit the wall-plate → eave strips.
	#[arg(long, default_value_t = false)]
	pub no_walls: bool,
	/// Omit half-hip facets (open gable-style ends if `--gables`).
	#[arg(long, default_value_t = false)]
	pub no_hips: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl PitchedRectangularRoof {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::PitchedRectangularRoof {
				footprint_x: self.footprint_x,
				footprint_z: self.footprint_z,
				ridge_height: self.ridge_height,
				eave_height: self.eave_height,
				ridge_inset: self.ridge_inset,
				gables: self.gables,
				no_walls: self.no_walls,
				no_hips: self.no_hips,
			},
			self.transform.transform(),
		)
	}
}
