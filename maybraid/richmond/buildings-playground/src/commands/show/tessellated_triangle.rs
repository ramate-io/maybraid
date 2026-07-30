//! `/show tessellated-triangle`

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TessellatedTriangle {
	/// Corner A (world XYZ).
	#[arg(long, value_delimiter = ',', num_args = 3, default_value = "0,0,0")]
	pub a: Vec<f32>,
	/// Corner B (world XYZ).
	#[arg(long, value_delimiter = ',', num_args = 3, default_value = "2,0,0")]
	pub b: Vec<f32>,
	/// Corner C (world XYZ).
	#[arg(long, value_delimiter = ',', num_args = 3, default_value = "0,0,-2")]
	pub c: Vec<f32>,
	/// Suggested kit leg length.
	#[arg(long, default_value_t = 1.0)]
	pub tile_width: f32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl TessellatedTriangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::TessellatedTriangle {
				a: vec3_from_args(&self.a),
				b: vec3_from_args(&self.b),
				c: vec3_from_args(&self.c),
				tile_width: self.tile_width,
			},
			self.transform.transform(),
		)
	}
}

fn vec3_from_args(v: &[f32]) -> Vec3 {
	Vec3::new(
		v.first().copied().unwrap_or(0.0),
		v.get(1).copied().unwrap_or(0.0),
		v.get(2).copied().unwrap_or(0.0),
	)
}
