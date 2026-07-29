//! `/show rectangular-half-gable`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RectangularHalfGable {
	/// Number of unit squares along the ridge (Z).
	#[arg(long, default_value_t = 4)]
	pub length_units: u32,
	/// Pitch about local +X in degrees.
	#[arg(long, default_value_t = 30.0)]
	pub pitch_degrees: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl RectangularHalfGable {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::RectangularHalfGable {
				length_units: self.length_units,
				pitch_degrees: self.pitch_degrees,
			},
			self.transform.transform(),
		)
	}
}
