//! `/show pathological-connecting-stairwell-gallery` — labeled I / L / U review set.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PathologicalConnectingStairwellGallery {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl PathologicalConnectingStairwellGallery {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::PathologicalConnectingStairwellGallery, self.transform.transform())
	}
}
