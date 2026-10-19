//! CPU-side playback uniform and representation mode.

use bevy::prelude::*;
use bevy::render::render_resource::ShaderType;

use crate::dust::bounds::DustBounds;
use crate::dust::disturbance::DustDisturbance;

/// Authored capture plane basis for representation B.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustCaptureBasis {
	pub right: Vec3,
	pub up: Vec3,
	pub centre: Vec3,
}

impl DustCaptureBasis {
	pub fn packed_right(self) -> Vec4 {
		self.right.extend(0.0)
	}

	pub fn packed_up(self) -> Vec4 {
		self.up.extend(0.0)
	}

	pub fn packed_centre(self) -> Vec4 {
		self.centre.extend(0.0)
	}
}

/// Optical and sampling overrides packed into `experiment`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DustExperiment {
	pub nearest: bool,
	pub density_multiplier: f32,
	pub boundary_exclusion: bool,
}

impl DustExperiment {
	pub fn packed(self) -> Vec4 {
		Vec4::new(
			if self.nearest { 1.0 } else { 0.0 },
			self.density_multiplier,
			if self.boundary_exclusion { 1.0 } else { 0.0 },
			0.0,
		)
	}
}

/// B = fixed plane, B+ = eight-view ring, C = animated density.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DustRepresentation {
	Plane,
	MultiView,
	Density,
}

impl DustRepresentation {
	pub fn from_label(label: &str) -> Option<Self> {
		match label {
			"B" => Some(Self::Plane),
			"B+" => Some(Self::MultiView),
			"C" => Some(Self::Density),
			_ => None,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Plane => "B",
			Self::MultiView => "B+",
			Self::Density => "C",
		}
	}

	/// Uniform `time.w`: 0 = B, 1 = C, 2 = B+.
	pub fn mode(self) -> f32 {
		match self {
			Self::Plane => 0.0,
			Self::Density => 1.0,
			Self::MultiView => 2.0,
		}
	}

	pub fn animation_gpu_bytes(self) -> usize {
		match self {
			Self::Density => 12_845_056,
			Self::MultiView => crate::dust::DustMultiView::GPU_BYTES,
			Self::Plane => 13_660_416,
		}
	}
}

/// GPU uniform. Layout must stay aligned with `shader.wgsl`.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct DustPlayback {
	pub time: Vec4,      // i, j, blend, mode(0=B,1=C,2=B+)
	pub settings: Vec4,  // group count, ray steps, sigmaT, active
	pub influence: Vec4, // world xyz, smoothed movement strength
	pub velocity: Vec4,  // xyz, radius
	pub origins: [Vec4; 10],
	pub right: Vec4,
	pub up: Vec4,
	pub centre: Vec4,
	pub experiment: Vec4, // nearest-view, optical density, boundary exclusion, reserved
}

impl DustPlayback {
	pub const SIGMA_T: f32 = 2.2;
	pub const MAX_GROUPS: usize = 10;

	pub fn new(
		representation: DustRepresentation,
		groups: u32,
		steps: u32,
		capture: DustCaptureBasis,
		experiment: DustExperiment,
	) -> Self {
		Self {
			time: Vec4::new(0.0, 1.0, 0.0, representation.mode()),
			settings: Vec4::new(groups as f32, steps as f32, Self::SIGMA_T, 0.0),
			influence: Vec4::ZERO,
			velocity: Vec4::new(0.0, 0.0, 0.0, DustDisturbance::RADIUS),
			origins: [Vec4::ZERO; Self::MAX_GROUPS],
			right: capture.packed_right(),
			up: capture.packed_up(),
			centre: capture.packed_centre(),
			experiment: experiment.packed(),
		}
	}

	pub fn set_source_time(&mut self, t: f32) {
		let frames = DustBounds::frames(t);
		self.time.x = frames.current as f32;
		self.time.y = frames.next as f32;
		self.time.z = frames.blend;
	}

	pub fn set_active(&mut self, active: bool) {
		self.settings.w = if active { 1.0 } else { 0.0 };
	}

	pub fn set_disturbance(&mut self, field: DustDisturbance) {
		self.influence = field.packed_influence();
		self.velocity = field.packed_velocity();
	}

	pub fn set_group_origin(&mut self, index: usize, origin: Vec3) {
		if index < Self::MAX_GROUPS {
			self.origins[index] = origin.extend(0.0);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn labels_map_to_shader_modes() {
		assert_eq!(DustRepresentation::from_label("B").unwrap().mode(), 0.0);
		assert_eq!(DustRepresentation::from_label("C").unwrap().mode(), 1.0);
		assert_eq!(DustRepresentation::from_label("B+").unwrap().mode(), 2.0);
		assert_eq!(DustRepresentation::from_label("nope"), None);
	}

	#[test]
	fn set_source_time_writes_interpolated_frames() {
		let capture =
			DustCaptureBasis { right: Vec3::X, up: Vec3::Y, centre: DustBounds::QUAD_CENTRE };
		let mut playback = DustPlayback::new(
			DustRepresentation::Density,
			1,
			96,
			capture,
			DustExperiment { nearest: false, density_multiplier: 1.0, boundary_exclusion: false },
		);
		playback.set_source_time(0.1);
		assert_eq!(playback.time.x, 1.0);
		assert_eq!(playback.time.y, 2.0);
		assert!((playback.time.z - 0.5).abs() < 1e-6);
		playback.set_active(true);
		assert_eq!(playback.settings.w, 1.0);
	}
}
