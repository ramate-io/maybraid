//! `/show half-triangular-hip`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct HalfTriangularHip {
	/// Pitch about local +X in degrees.
	#[arg(long, default_value_t = 30.0)]
	pub pitch_degrees: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl HalfTriangularHip {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::HalfTriangularHip {
				pitch_degrees: self.pitch_degrees,
			},
			self.transform.transform(),
		)
	}
}
