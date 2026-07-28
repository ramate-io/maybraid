//! `/show header-90`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Header90 {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Header90 {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::Header90, self.transform.transform())
	}
}
