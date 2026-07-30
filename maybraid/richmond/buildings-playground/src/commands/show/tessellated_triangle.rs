//! `/show tessellated-triangle`

use bevy::prelude::*;
use bevy_math::Vec2;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TessellatedTriangle {
	/// Corner A in panel \(X,Z\).
	#[arg(long, value_delimiter = ',', num_args = 2, default_value = "0,0")]
	pub a: Vec<f32>,
	/// Corner B in panel \(X,Z\).
	#[arg(long, value_delimiter = ',', num_args = 2, default_value = "2,0")]
	pub b: Vec<f32>,
	/// Corner C in panel \(X,Z\).
	#[arg(long, value_delimiter = ',', num_args = 2, default_value = "0,2")]
	pub c: Vec<f32>,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl TessellatedTriangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::TessellatedTriangle {
				a: vec2_from_args(&self.a),
				b: vec2_from_args(&self.b),
				c: vec2_from_args(&self.c),
			},
			self.transform.transform(),
		)
	}
}

fn vec2_from_args(v: &[f32]) -> Vec2 {
	Vec2::new(v.first().copied().unwrap_or(0.0), v.get(1).copied().unwrap_or(0.0))
}
