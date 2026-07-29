//! Circular joint tile and LOD content policy (high + mid only under a parent level).

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;

use crate::assets::AssetPath;
use crate::assets::partitions::rough_stonework::{JOINT_HIGH, JOINT_MID};
use crate::partitions::geometry::polyline::wrap_pi;
use crate::partitions::geometry::PartitionTile;
use crate::partitions::host::{posed_asset_tier, warm_host};
use crate::partitions::probe::{PartitionLodBand, PartitionLodProbe};
use crate::placed::{Placement, Placed};

/// Joint kit half-extent in \(X/Z\) (\([-0.5, 0.5]\)).
pub const JOINT_KIT_HALF: f32 = 0.5;
/// Base world radius when the kink is purely planar.
pub const JOINT_BASE_RADIUS: f32 = 0.15;
/// Extra world radius per radian of vertical (slope) kink.
pub const JOINT_RADIUS_PER_SLOPE_RAD: f32 = 0.55;

/// Tighter lone-joint leaf banding (playground). Under a polyline parent, joints
/// follow the parent level instead.
pub const JOINT_HIGH_FACTOR: f32 = 3.0;
pub const JOINT_MEDIUM_FACTOR: f32 = 12.0;

/// Circular / post joint between upright linear partition segments.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct JointPartition;

impl JointPartition {
	/// Placement for a joint at `cur` bridging inbound/outbound slope (and plan) angles.
	pub fn placed_at(
		cur: Vec3,
		yaw_in: f32,
		yaw_out: f32,
		roll_in: f32,
		roll_out: f32,
	) -> Placed<PartitionTile> {
		let droll = (roll_out - roll_in).abs();
		let radius = JOINT_BASE_RADIUS + JOINT_RADIUS_PER_SLOPE_RAD * droll;
		let xz = (radius / JOINT_KIT_HALF).max(1e-4);
		let yaw = yaw_in + 0.5 * wrap_pi(yaw_out - yaw_in);
		let roll = 0.5 * (roll_in + roll_out);
		Placed {
			geom: PartitionTile::Joint,
			placement: Placement::new(cur, yaw)
				.with_roll(roll)
				.with_scale(Vec3::new(xz, 1.0, xz)),
		}
	}
}

/// Joint mesh choice under a parent level; optional lone-leaf host.
pub struct JointLod;

impl JointLod {
	pub fn included_at(level: LodSceneLevel) -> bool {
		matches!(level, LodSceneLevel::High | LodSceneLevel::Medium)
	}

	pub fn asset_for_level(level: LodSceneLevel) -> Option<AssetPath> {
		match level {
			LodSceneLevel::High => Some(JOINT_HIGH),
			LodSceneLevel::Medium => Some(JOINT_MID),
			_ => None,
		}
	}

	pub fn band_from_distance_factor(factor: f32) -> PartitionLodBand {
		if factor <= JOINT_HIGH_FACTOR {
			PartitionLodBand::High
		} else if factor <= JOINT_MEDIUM_FACTOR {
			PartitionLodBand::Medium
		} else {
			PartitionLodBand::Low
		}
	}

	pub fn posed_tier(transform: Transform, level: LodSceneLevel) -> impl Scene + 'static {
		posed_asset_tier(Self::asset_for_level(level), transform)
	}

	pub fn leaf_host(lod_ref: &LodRef) -> impl Scene + 'static {
		let probe = PartitionLodProbe::from_aabb(lod_ref.bounds);
		let factor = lod_ref
			.current_transform
			.translation
			.distance(probe.center)
			/ probe.extent;
		let level = match Self::band_from_distance_factor(factor) {
			PartitionLodBand::High => LodSceneLevel::High,
			PartitionLodBand::Medium => LodSceneLevel::Medium,
			PartitionLodBand::Low | PartitionLodBand::UltraLow => LodSceneLevel::Low,
		};
		warm_host(
			level,
			probe,
			Transform::IDENTITY,
			[
				(LodSceneLevel::High, Some(JOINT_HIGH)),
				(LodSceneLevel::Medium, Some(JOINT_MID)),
				(LodSceneLevel::Low, None),
			],
		)
	}
}
