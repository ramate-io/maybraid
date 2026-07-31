//! `/show rectangular-orthonormal-tube` — rectangular cross-section polyline → four clipped rect strips.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RectangularOrthonormalTube {
	/// Uniform inset on the left-wall middle bay (`0` = solid).
	#[arg(long, default_value_t = 0.35)]
	pub inset: f32,
	/// Shared bank about the path tangent (radians).
	#[arg(long, default_value_t = 0.15)]
	pub roll: f32,
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl RectangularOrthonormalTube {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::RectangularOrthonormalTube {
				inset: self.inset,
				roll: self.roll,
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
			},
			self.transform.transform(),
		)
	}
}
