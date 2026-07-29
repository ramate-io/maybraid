//! `/show polyline-wall` — portal-sensitive [`PolylineWall`] under [`Walling`].

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PolylineWall {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl PolylineWall {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::PolylineWall, self.transform.transform())
	}
}
