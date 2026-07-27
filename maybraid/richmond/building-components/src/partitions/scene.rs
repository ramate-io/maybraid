//! Partition geometry components → rough-stone scene components.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::world_serialization::WorldAssetRoot;
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::partitions::rough_stonework::{ARC_180, ARC_90, HEADER_90, LINEAR};
use crate::assets::AssetPath;
use crate::partitions::geometry::Wall;
use crate::partitions::geometry_components::WallComponent;
use crate::partitions::rough_stonework::{
	RoughStonework15, RoughStonework180, RoughStonework90, RoughStoneworkHeader15,
	RoughStoneworkHeader180, RoughStoneworkHeader90, RoughStoneworkLinear,
	RoughStoneworkLinearHeaderSubsegment, RoughStoneworkLinearSubsegment,
};
use crate::placed::Placed;
use crate::scene_children;

fn pose(translation: Vec3, yaw: f32, scale: Vec3) -> Transform {
	Transform::from_translation(translation)
		.with_rotation(Quat::from_rotation_y(yaw))
		.with_scale(scale)
}

fn posed_glb(asset: AssetPath, transform: Transform) -> impl Scene + 'static {
	let path = asset.gltf_scene_0();
	bsn! {
		WorldAssetRoot({path})
		template_value(transform)
	}
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
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

fn posed_wall_component(comp: WallComponent, transform: Transform, lod_ref: &LodRef) -> Box<dyn Scene> {
	match comp {
		WallComponent::Linear => Box::new(posed_glb(LINEAR, transform)),
		WallComponent::Arc180 => Box::new(posed_glb(ARC_180, transform)),
		WallComponent::Arc90 => Box::new(posed_glb(ARC_90, transform)),
		WallComponent::HeaderArc90 => Box::new(posed_glb(HEADER_90, transform)),
		other => Box::new(with_pose(transform, wall_component_scene(other, lod_ref))),
	}
}

/// Rough-stone scene for placed continuous wall geometry.
pub fn rough_stone_wall(placed: &Placed<Wall>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw, piece.scale);
			posed_wall_component(piece.geom, transform, lod_ref)
		})
		.collect();
	scene_children(children)
}
