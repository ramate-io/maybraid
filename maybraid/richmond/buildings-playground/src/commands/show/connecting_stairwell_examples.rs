//! `/show connecting-stairwell-examples` — pathological circular-spiral wells.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingStairwellExamples {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ConnectingStairwellExamples {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::ConnectingStairwellExamples, self.transform.transform())
	}
}
