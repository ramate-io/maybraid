//! `/show linear-wall` — portal-sensitive [`LinearWall`] under [`Walling`].

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LinearWall {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl LinearWall {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::LinearWall, self.transform.transform())
	}
}
