//! Portal-sensitive polyline wall whose path is sampled with allowed-angle noise.
//!
//! Uses [`procedural_common::noisy_path`] to spend a distance budget under per-axis
//! turn limits, then builds a [`super::PolylineWall`] from the resulting points.

use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::{AllowedAngles, NoiseParams, NoisyPathParams, StepLenRange};
use richmond_building_components::partitions::{
	DEFAULT_MIN_JOINT_ANGLE, DEFAULT_TILE_WIDTH, PartitionNode,
};
use richmond_building_components::{BuildingComponents};

use crate::walling::polyline::{PolylineWall, PolylineWallParams, DEFAULT_PORTAL_WIDTH};
use crate::walling::portal::{AssignedPortal, MustAssignPortal, WallRegion};

const DEFAULT_THICK: f32 = 0.15 / 0.2;

/// Parameters for [`NoisyPolylineWall::new`].
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyPolylineWallParams {
	pub start: Vec3,
	pub initial_dir: Vec3,
	/// Total path length budget.
	pub distance: f32,
	pub step_len: StepLenRange,
	/// Allowed pitch / yaw / roll (see [`AllowedAngles`]).
	pub allowed_angles: AllowedAngles,
	/// Seed / frequency for the path walk.
	pub path_noise: NoiseParams,
	pub height: f32,
	pub thickness: f32,
	pub portal_width: f32,
	/// Suggested tile width along each solid edge; fitted so \(n\) tiles span exactly.
	pub tile_width: f32,
	/// Omit joints when plan/slope kinks are below this (radians).
	pub min_joint_angle: f32,
	pub must_assign: Vec<MustAssignPortal>,
	pub must_not_assign: Vec<WallRegion>,
	pub portal_noise: NoiseParams,
	pub optional_portals: (u32, u32),
}

impl Default for NoisyPolylineWallParams {
	fn default() -> Self {
		Self {
			start: Vec3::ZERO,
			initial_dir: Vec3::Z,
			distance: 12.0,
			step_len: StepLenRange::new(0.75, 1.25),
			allowed_angles: AllowedAngles::yaw_pitch(
				std::f32::consts::FRAC_PI_6,
				std::f32::consts::FRAC_PI_8,
			),
			path_noise: NoiseParams { seed: 1337, frequency: 0.35, ..NoiseParams::default() },
			height: 3.0,
			thickness: DEFAULT_THICK,
			portal_width: DEFAULT_PORTAL_WIDTH,
			tile_width: DEFAULT_TILE_WIDTH,
			min_joint_angle: DEFAULT_MIN_JOINT_ANGLE,
			must_assign: vec![],
			must_not_assign: vec![],
			portal_noise: NoiseParams::default(),
			optional_portals: (0, 0),
		}
	}
}

/// Noisy 3D polyline wall (joinery / vertical-angle test harness).
#[derive(Debug, Clone, PartialEq)]
pub struct NoisyPolylineWall {
	pub path_noise: NoiseParams,
	pub allowed_angles: AllowedAngles,
	pub distance: f32,
	pub step_len: StepLenRange,
	pub points: Vec<Vec3>,
	pub wall: PolylineWall,
}

impl NoisyPolylineWall {
	pub fn new(params: NoisyPolylineWallParams) -> Self {
		let step_len = StepLenRange::new(params.step_len.min, params.step_len.max);
		let points = NoisyPathParams {
			start: params.start,
			initial_dir: params.initial_dir,
			distance: params.distance,
			step_len,
			allowed_angles: params.allowed_angles,
			noise: params.path_noise,
		}
		.generate();

		let wall = PolylineWall::new(PolylineWallParams {
			points: points.clone(),
			height: params.height,
			thickness: params.thickness,
			portal_width: params.portal_width,
			tile_width: params.tile_width,
			min_joint_angle: params.min_joint_angle,
			must_assign: params.must_assign,
			must_not_assign: params.must_not_assign,
			portal_noise: params.portal_noise,
			optional_portals: params.optional_portals,
		});

		Self {
			path_noise: params.path_noise,
			allowed_angles: params.allowed_angles,
			distance: params.distance.max(0.0),
			step_len,
			points,
			wall,
		}
	}

	pub fn portals(&self) -> &[AssignedPortal] {
		&self.wall.portals
	}

	pub fn partitions(&self) -> &[richmond_building_components::partitions::PartitionNode] {
		&self.wall.partitions
	}
}

impl BuildingComponents for NoisyPolylineWall {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Vec<PartitionNode> {
		self.wall.partitions.clone()
	}
}


#[cfg(test)]
mod tests {
	use super::*;
	use crate::walling::portal::Portal;
	use richmond_building_components::partitions::Partition;

	#[test]
	fn builds_wall_from_noisy_path() -> anyhow::Result<()> {
		let noisy = NoisyPolylineWall::new(NoisyPolylineWallParams {
			distance: 10.0,
			step_len: StepLenRange::new(0.75, 1.25),
			allowed_angles: AllowedAngles::yaw_pitch(0.4, 0.2),
			path_noise: NoiseParams { seed: 9, ..NoiseParams::default() },
			must_assign: vec![MustAssignPortal::at(0.5, Portal::Window)],
			optional_portals: (0, 0),
			..NoisyPolylineWallParams::default()
		});
		assert!(noisy.points.len() >= 2);
		assert!(!noisy.wall.partitions.is_empty());
		assert!(noisy
			.wall
			.partitions
			.iter()
			.any(|p| matches!(p.geometry, Partition::Polyline(_))));
		assert_eq!(noisy.portals().len(), 1);
		Ok(())
	}
}
