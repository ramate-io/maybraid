//! `/show clipped-rectangle` — best-fit rectangle with a closed clip cutout.

use bevy::prelude::*;
use clap::Args;

use super::transform::{parse_vec3_csv, parse_vec3_polyline, Vec3Polyline};
use super::ShowTransform;
use crate::preview::PreviewSubject;

const DEFAULT_CLIP: &str = "0.6,0,0.3;1.4,0,0.3;1.4,0,0.7;0.6,0,0.7";

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ClippedRectangle {
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a0: Vec3,
	#[arg(long, default_value = "2,0,0", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub a1: Vec3,
	#[arg(long, default_value = "0.1,0.2,1", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b0: Vec3,
	#[arg(long, default_value = "2.2,-0.1,1.1", value_parser = parse_vec3_csv, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z")]
	pub b1: Vec3,
	#[arg(long, default_value = DEFAULT_CLIP, value_parser = parse_vec3_polyline, allow_hyphen_values = true)]
	#[arg(value_name = "X,Y,Z;…")]
	pub clip: Vec3Polyline,
	#[arg(long, default_value_t = 0.1)]
	pub min_dihedral: f32,
	#[arg(long, default_value_t = false)]
	pub no_joint: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl ClippedRectangle {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::ClippedRectangle {
				a0: self.a0,
				a1: self.a1,
				b0: self.b0,
				b1: self.b1,
				clip: self.clip.0,
				min_dihedral: self.min_dihedral,
				no_joint: self.no_joint,
			},
			self.transform.transform(),
		)
	}
}
