//! Distance / extent probe for stick LOD hosts.

use bevy::prelude::{Component, Query, Transform, With};
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use lod::scene::{LodSceneLevel, LodSceneStatus};

use crate::lod_band::{characteristic_extent_abs, placement_center, DistanceLodBand};
use crate::placed::Placement;
use crate::sticks::geometry::StickGeometry;

/// `distance / radius` (or trunk extent) at or below this → High stick mesh.
pub const STICK_HIGH_FACTOR: f32 = 10.0;
/// `distance / radius` (or trunk extent) at or below this → Medium stick mesh.
pub const STICK_MEDIUM_FACTOR: f32 = 50.0;
/// `distance / radius` (or trunk extent) at or below this → Low stick mesh; beyond → UltraLow (empty).
pub const STICK_LOW_FACTOR: f32 = 200.0;

#[derive(Debug, Clone, Copy, Component, Default)]
pub struct StickLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl StickLodProbe {
	/// Extent is **radius/girth** for segments; trunks keep the max-axis characteristic extent
	/// (typically length-dominated).
	pub fn from_stick(placement: &Placement, geometry: StickGeometry) -> Self {
		let extent = match geometry {
			StickGeometry::Trunk => characteristic_extent_abs(placement),
			StickGeometry::Segment => {
				placement.scale.x.abs().max(placement.scale.z.abs()).max(1e-4)
			}
		};
		Self { center: placement_center(placement), extent }
	}

	fn band_to_level(band: DistanceLodBand) -> LodSceneLevel {
		match band {
			DistanceLodBand::High => LodSceneLevel::High,
			DistanceLodBand::Medium => LodSceneLevel::Medium,
			DistanceLodBand::Low => LodSceneLevel::Low,
			// Stick mesh UltraLow is a real empty tier (not collapsed onto Low).
			DistanceLodBand::UltraLow => LodSceneLevel::UltraLow,
		}
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		let distance = viewer.translation.distance(self.center);
		let factor = distance / self.extent.max(1e-4);
		Self::band_to_level(DistanceLodBand::from_factors(
			factor,
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		))
	}

	/// Nearest viewer pose in 3D (the same metric as [`Self::level_for`]).
	pub fn nearest_pose<'a>(
		&self,
		poses: impl IntoIterator<Item = &'a lod::LodNodePose>,
	) -> Option<&'a lod::LodNodePose> {
		lod::LodNodePose::nearest_3d(poses, self.center)
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = Self::band_to_level(DistanceLodBand::from_factors(
			lod_ref.previous_transform.translation.distance(self.center) / self.extent.max(1e-4),
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		));
		let curr = Self::band_to_level(DistanceLodBand::from_factors(
			lod_ref.current_transform.translation.distance(self.center) / self.extent.max(1e-4),
			STICK_HIGH_FACTOR,
			STICK_MEDIUM_FACTOR,
			STICK_LOW_FACTOR,
		));
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}
}

/// Update stick host levels from the [`lod::LodViewer`] pose.
pub fn update_stick_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&StickLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	if viewer.is_empty() {
		return;
	}
	for (probe, mut level) in &mut hosts {
		let Some(pose) = probe.nearest_pose(viewer.iter()) else {
			continue;
		};
		let next = probe.level_for(&pose.current);
		if *level != next {
			*level = next;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::placed::Placement;
	use crate::sticks::node::StickNode;
	use bevy::prelude::{Entity, Transform};
	use bevy::shape::Aabb3d;
	use lod::lod_ref::LodRef;
	use lod::scene::LodScene;

	fn lod_ref_at<'a>(at: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: at,
			current_transform: at,
			bounds,
		}
	}

	#[test]
	fn three_d_nearest_keeps_the_five_meter_ground_ref() -> anyhow::Result<()> {
		let probe = StickLodProbe { center: Vec3::ZERO, extent: 1.0 };
		let ground = lod::LodNodePose::settled(Transform::from_xyz(5.0, 0.0, 0.0));
		let map = lod::LodNodePose::settled(Transform::from_xyz(0.0, 420.0, 0.0));
		let Some(pose) = probe.nearest_pose([&ground, &map]) else {
			return Err(anyhow::anyhow!("3D nearest pose"));
		};
		assert_eq!(pose.current.translation, ground.current.translation);
		assert_eq!(probe.level_for(&pose.current), LodSceneLevel::High);
		let Some(xz) = lod::LodNodePose::nearest([&ground, &map], probe.center) else {
			return Err(anyhow::anyhow!("XZ nearest pose"));
		};
		assert_eq!(xz.current.translation, map.current.translation);

		let node = StickNode::segment(Placement::new(Vec3::ZERO, 0.0));
		let ground_t = Transform::from_xyz(5.0, 0.0, 0.0);
		let map_t = Transform::from_xyz(0.0, 420.0, 0.0);
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let refs = [lod_ref_at(&ground_t, &bounds), lod_ref_at(&map_t, &bounds)];
		assert_eq!(
			node.scene_lod_nearest(&refs, Vec3::ZERO)
				.map(|r| r.current_transform.translation),
			Some(ground_t.translation)
		);
		assert_eq!(node.scene_lod_level_from_nearest(&refs, Vec3::ZERO), LodSceneLevel::High);
		Ok(())
	}
}
