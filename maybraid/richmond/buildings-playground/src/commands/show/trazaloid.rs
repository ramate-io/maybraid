//! `/show trazaloid` — two-band trapezoidal-pyramid shell.

use bevy::prelude::*;
use clap::Args;

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct Trazaloid {
	#[arg(long, default_value_t = 8.0)]
	pub footprint_x: f32,
	#[arg(long, default_value_t = 6.0)]
	pub footprint_z: f32,
	#[arg(long, default_value_t = 4.0)]
	pub ridge_x: f32,
	#[arg(long, default_value_t = 3.0)]
	pub ridge_z: f32,
	#[arg(long, default_value_t = 3.0)]
	pub lower_height: f32,
	#[arg(long, default_value_t = 2.5)]
	pub upper_height: f32,
	#[arg(long, default_value_t = 0.35)]
	pub band_vertical_offset: f32,
	#[arg(long, default_value_t = 0.25)]
	pub waist_horizontal_offset: f32,
	#[arg(long, default_value_t = false)]
	pub door_north: bool,
	#[arg(long, default_value_t = false)]
	pub door_east: bool,
	#[arg(long, default_value_t = true)]
	pub door_south: bool,
	#[arg(long, default_value_t = false)]
	pub door_west: bool,
	#[arg(long, default_value_t = 0.28)]
	pub door_width_frac: f32,
	/// Absolute door opening width in meters (`> 0` overrides `--door-width-frac`).
	#[arg(long, default_value_t = 1.2)]
	pub door_thickness: f32,
	#[arg(long, default_value_t = 0.7)]
	pub door_height_frac: f32,
	#[arg(long, default_value_t = 2)]
	pub face_post_count: u32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl Trazaloid {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::Trazaloid {
				footprint_x: self.footprint_x,
				footprint_z: self.footprint_z,
				ridge_x: self.ridge_x,
				ridge_z: self.ridge_z,
				lower_height: self.lower_height,
				upper_height: self.upper_height,
				band_vertical_offset: self.band_vertical_offset,
				waist_horizontal_offset: self.waist_horizontal_offset,
				door_north: self.door_north,
				door_east: self.door_east,
				door_south: self.door_south,
				door_west: self.door_west,
				door_width_frac: self.door_width_frac,
				door_thickness: self.door_thickness,
				door_height_frac: self.door_height_frac,
				face_post_count: self.face_post_count,
			},
			self.transform.transform(),
		)
	}
}
