//! Partition geometry components → rough-stone scene components.

use bevy::prelude::{Children, Transform};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::partitions::geometry::Wall;
use crate::partitions::geometry_components::WallComponent;
use crate::partitions::rough_stonework::{
	RoughStonework15, RoughStonework180, RoughStonework90, RoughStoneworkHeader15,
	RoughStoneworkHeader180, RoughStoneworkHeader90, RoughStoneworkLinear,
	RoughStoneworkLinearHeaderSubsegment, RoughStoneworkLinearSubsegment,
};
use crate::placed::Placed;

fn pose(translation: Vec3, yaw: f32) -> Transform {
	Transform::from_translation(translation).with_rotation(Quat::from_rotation_y(yaw))
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	bsn! {
		template_value(transform)
		Children [ ({child}) ]
	}
}

pub fn wall_component_scene(comp: WallComponent, lod_ref: &LodRef) -> Box<dyn Scene> {
	match comp {
		WallComponent::Linear => Box::new(RoughStoneworkLinear::from(comp).scene_with_lod(lod_ref)),
		WallComponent::LinearSubsegment => {
			Box::new(RoughStoneworkLinearSubsegment::from(comp).scene_with_lod(lod_ref))
		}
		WallComponent::LinearHeaderSubsegment => {
			Box::new(RoughStoneworkLinearHeaderSubsegment::from(comp).scene_with_lod(lod_ref))
		}
		WallComponent::Arc180 => Box::new(RoughStonework180::from(comp).scene_with_lod(lod_ref)),
		WallComponent::Arc90 => Box::new(RoughStonework90::from(comp).scene_with_lod(lod_ref)),
		WallComponent::Arc15 => Box::new(RoughStonework15::from(comp).scene_with_lod(lod_ref)),
		WallComponent::HeaderArc180 => {
			Box::new(RoughStoneworkHeader180::from(comp).scene_with_lod(lod_ref))
		}
		WallComponent::HeaderArc90 => {
			Box::new(RoughStoneworkHeader90::from(comp).scene_with_lod(lod_ref))
		}
		WallComponent::HeaderArc15 => {
			Box::new(RoughStoneworkHeader15::from(comp).scene_with_lod(lod_ref))
		}
	}
}

/// Rough-stone scene for placed continuous wall geometry.
pub fn rough_stone_wall(placed: &Placed<Wall>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw);
			let child = wall_component_scene(piece.geom, lod_ref);
			Box::new(with_pose(transform, child)) as Box<dyn Scene>
		})
		.collect();
	bsn! {
		Children [ {children} ]
	}
}
