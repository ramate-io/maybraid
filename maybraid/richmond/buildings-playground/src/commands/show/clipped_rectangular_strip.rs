//! `/show clipped-rectangular-strip` — two-rail rectangle strip with a mid-bay inset.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ClippedRectangularStrip {
	#[arg(long, default_value_t = 0.35)]
	pub inset: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ClippedRectangularStrip {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ClippedRectangularStrip { inset: self.inset },
			self.transform.transform(),
		)
	}
}
