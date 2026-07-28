//! `/show bedroom`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Bedroom {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Bedroom {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::Bedroom, self.transform.transform())
	}
}
