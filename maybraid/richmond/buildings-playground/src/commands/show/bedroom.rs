//! `/show bedroom`

use bevy::prelude::*;
use clap::Args;

use super::transform::parse_vec3_csv;
use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Bedroom {
	/// Cell AABB size `x,y,z` in world units (origin at min corner).
	#[arg(long, default_value = "4,3,3.5", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub extent: Vec3,
	/// Unit noise sample in \[0, 1\] for layout fitting.
	#[arg(long, default_value_t = 0.5)]
	pub noise: f32,
	/// Place a required door circulation region on the −Z face (exclusion demo).
	#[arg(long, default_value_t = false)]
	pub door: bool,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Bedroom {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::Bedroom {
				extent: self.extent.max(Vec3::splat(1e-4)),
				noise: self.noise.clamp(0.0, 1.0),
				door: self.door,
			},
			self.transform.transform(),
		)
	}
}
