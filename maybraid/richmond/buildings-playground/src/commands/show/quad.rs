//! `/show quad`

use bevy::prelude::*;
use clap::Args;
use richmond_building_components::panels::DEFAULT_TILE_WIDTH;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Quad {
	/// Depth / run (top at \(Z = 0\), bottom at \(Z = -\texttt{depth}\)).
	#[arg(long, default_value_t = 2.0)]
	pub depth: f32,
	/// Rectangular span along X. Omit for ends-only.
	#[arg(long)]
	pub length: Option<f32>,
	/// Suggested tile width along X (fitted to `length`).
	#[arg(long, default_value_t = DEFAULT_TILE_WIDTH)]
	pub tile_width: f32,
	/// Optional suggested tile depth along Z (fitted so body tiles span `depth`).
	#[arg(long)]
	pub tile_height: Option<f32>,
	/// Left end-triangle base. Positive = upright, negative = flipped. Omit for none.
	#[arg(long, allow_negative_numbers = true)]
	pub left: Option<f32>,
	/// Right end-triangle base. Positive = upright, negative = flipped. Omit for none.
	#[arg(long, allow_negative_numbers = true)]
	pub right: Option<f32>,
	/// Top edge-triangle base. Positive = upright, negative = flipped. Omit for none.
	#[arg(long, allow_negative_numbers = true)]
	pub top: Option<f32>,
	/// Bottom edge-triangle base. Positive = upright, negative = flipped. Omit for none.
	#[arg(long, allow_negative_numbers = true)]
	pub bottom: Option<f32>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Quad {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::Quad {
				depth: self.depth,
				length: self.length,
				tile_width: self.tile_width,
				tile_height: self.tile_height,
				left: self.left,
				right: self.right,
				top: self.top,
				bottom: self.bottom,
			},
			self.transform.transform(),
		)
	}
}
