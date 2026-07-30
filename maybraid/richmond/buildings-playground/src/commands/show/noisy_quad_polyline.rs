//! `/show noisy-quad-polyline` — noisy path as `PartitionGeometry::QuadPolyline` with uniform roll.

use bevy::prelude::*;
use clap::Args;
use procedural_common::{AllowedAngles, NoiseParams, StepLenRange};
use richmond_building_components::panels::{
	DEFAULT_MIN_EDGE_TRIANGLE_ANGLE, DEFAULT_MIN_JOINT_ANGLE, DEFAULT_TILE_WIDTH,
};

use super::ShowTransform;
use crate::preview::PreviewSubject;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct NoisyQuadPolyline {
	/// Uniform roll applied to every segment (radians).
	#[arg(long, default_value_t = 0.0)]
	pub roll: f32,
	/// Panel depth (wall height after stand-up).
	#[arg(long, default_value_t = 3.0)]
	pub depth: f32,
	/// Suggested tile width along each edge.
	#[arg(long, default_value_t = DEFAULT_TILE_WIDTH)]
	pub tile_width: f32,
	/// Omit joints when plan kinks are below this (radians).
	#[arg(long, default_value_t = DEFAULT_MIN_JOINT_ANGLE)]
	pub min_joint_angle: f32,
	/// Omit edge triangles when plan kinks are below this (radians).
	#[arg(long, default_value_t = DEFAULT_MIN_EDGE_TRIANGLE_ANGLE)]
	pub min_edge_triangle_angle: f32,
	/// Total path length budget.
	#[arg(long, default_value_t = 12.0)]
	pub distance: f32,
	/// Minimum segment length per step.
	#[arg(long, default_value_t = 0.75)]
	pub step_len_min: f32,
	/// Maximum segment length per step.
	#[arg(long, default_value_t = 1.25)]
	pub step_len_max: f32,
	/// Max absolute pitch from horizontal (radians).
	#[arg(long, default_value_t = 0.4)]
	pub max_angle_x: f32,
	/// Max per-step yaw (radians about world +Y).
	#[arg(long, default_value_t = 0.55)]
	pub max_angle_y: f32,
	/// Max per-step roll for the path walk (unused for positions).
	#[arg(long, default_value_t = 0.0)]
	pub max_angle_z: f32,
	/// Path noise seed.
	#[arg(long, default_value_t = 1337)]
	pub seed: i32,
	#[command(flatten)]
	pub transform: ShowTransform,
}

impl NoisyQuadPolyline {
	pub fn into_preview(self) -> (PreviewSubject, Transform) {
		(
			PreviewSubject::NoisyQuadPolyline {
				roll: self.roll,
				depth: self.depth.max(1e-4),
				tile_width: self.tile_width.max(1e-4),
				min_joint_angle: self.min_joint_angle.max(0.0),
				min_edge_triangle_angle: self.min_edge_triangle_angle.max(0.0),
				distance: self.distance.max(1e-3),
				step_len: StepLenRange::new(self.step_len_min, self.step_len_max),
				allowed_angles: AllowedAngles::new(
					self.max_angle_x,
					self.max_angle_y,
					self.max_angle_z,
				),
				path_noise: NoiseParams {
					seed: self.seed,
					frequency: 0.35,
					..NoiseParams::default()
				},
			},
			self.transform.transform(),
		)
	}
}
