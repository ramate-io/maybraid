//! `/show divided-paneling`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct DividedPaneling {
	/// Suggested kit leg length for tessellated triangles.
	#[arg(long, default_value_t = 1.0)]
	pub tile_width: f32,
	/// When set, use three nodes (two segments) instead of the default two.
	#[arg(long, default_value_t = false)]
	pub three_nodes: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl DividedPaneling {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::DividedPaneling {
				tile_width: self.tile_width,
				three_nodes: self.three_nodes,
			},
			self.transform.transform(),
		)
	}
}
