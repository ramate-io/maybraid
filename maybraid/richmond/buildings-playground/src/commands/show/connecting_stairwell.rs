//! `/show connecting-stairwell` — run-in floor + spiral flight between two openings.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ConnectingStairwell {
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ConnectingStairwell {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(PreviewSubject::ConnectingStairwell, self.transform.transform())
	}
}
