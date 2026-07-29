//! Circular joint tile and LOD content policy (high + mid only).

use bevy::prelude::{Children, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodSceneHost};

use crate::assets::AssetPath;
use crate::assets::partitions::rough_stonework::{JOINT_HIGH, JOINT_MID};
use crate::partitions::geometry::PartitionTile;
use crate::partitions::mesh_set::mesh_child;
use crate::partitions::probe::PartitionLodProbe;
use crate::placed::{Placement, Placed};

/// Joint kit half-extent in \(X/Z\) (\([-0.5, 0.5]\)).
pub const JOINT_KIT_HALF: f32 = 0.5;
/// Base world radius when the kink is purely planar.
pub const JOINT_BASE_RADIUS: f32 = 0.15;
/// Extra world radius per radian of vertical (slope) kink.
pub const JOINT_RADIUS_PER_SLOPE_RAD: f32 = 0.55;

/// Tighter lone-joint host banding (playground leaf). Under a polyline parent,
/// joints follow the **parent** level instead.
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
	) -> Placed<crate::partitions::geometry::PartitionTile> {
		use crate::partitions::geometry::polyline::wrap_pi;

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

/// Joint mesh choice / lone-host banding.
pub struct JointLod;

impl JointLod {
	/// Whether a parent `LodSceneLevel` should include joint mesh content.
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

	pub fn band_from_distance_factor(factor: f32) -> crate::partitions::probe::PartitionLodBand {
		use crate::partitions::probe::PartitionLodBand;
		if factor <= JOINT_HIGH_FACTOR {
			PartitionLodBand::High
		} else if factor <= JOINT_MEDIUM_FACTOR {
			PartitionLodBand::Medium
		} else {
			PartitionLodBand::Low
		}
	}

	pub fn posed_tier(transform: Transform, level: LodSceneLevel) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = match Self::asset_for_level(level) {
			Some(asset) => vec![mesh_child(asset)],
			None => vec![],
		};
		bsn! {
			template_value(transform)
			Visibility::Inherited
			Children [ {children} ]
		}
	}

	pub fn posed_host(
		transform: Transform,
		level: LodSceneLevel,
		probe: PartitionLodProbe,
	) -> impl Scene + 'static {
		let roots = vec![
			tier_or_empty(LodSceneLevel::High, JOINT_HIGH, level == LodSceneLevel::High),
			tier_or_empty(
				LodSceneLevel::Medium,
				JOINT_MID,
				level == LodSceneLevel::Medium,
			),
			empty_level_root(LodSceneLevel::Low, level == LodSceneLevel::Low),
		];
		let level_roots: Box<dyn Scene> = Box::new(bsn! {
			LodLevelRoots
			Transform::default()
			Visibility::Inherited
			Children [ {roots} ]
		});
		let host_children = vec![level_roots];
		bsn! {
			LodSceneHost
			template_value(level)
			template_value(probe)
			template_value(transform)
			Visibility::Inherited
			Children [ {host_children} ]
		}
	}

	pub fn leaf_host(lod_ref: &LodRef) -> impl Scene + 'static {
		use crate::partitions::probe::PartitionLodBand;
		let center = bevy_math::Vec3::from((lod_ref.bounds.min + lod_ref.bounds.max) * 0.5);
		let size = lod_ref.bounds.max - lod_ref.bounds.min;
		let extent = size.x.max(size.y).max(size.z).max(1e-4);
		let factor = lod_ref.current_transform.translation.distance(center) / extent;
		let band = Self::band_from_distance_factor(factor);
		let level = match band {
			PartitionLodBand::High => LodSceneLevel::High,
			PartitionLodBand::Medium => LodSceneLevel::Medium,
			PartitionLodBand::Low | PartitionLodBand::UltraLow => LodSceneLevel::Low,
		};
		let probe = PartitionLodProbe { center, extent };
		Self::posed_host(Transform::IDENTITY, level, probe)
	}
}

fn tier_or_empty(level: LodSceneLevel, asset: AssetPath, visible: bool) -> Box<dyn Scene> {
	let children = vec![mesh_child(asset)];
	let visibility = if visible {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	};
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}

fn empty_level_root(level: LodSceneLevel, visible: bool) -> Box<dyn Scene> {
	let children: Vec<Box<dyn Scene>> = vec![];
	let visibility = if visible {
		Visibility::Inherited
	} else {
		Visibility::Hidden
	};
	Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		template_value(visibility)
		Children [ {children} ]
	})
}
