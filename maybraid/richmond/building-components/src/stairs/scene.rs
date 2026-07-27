//! Stair geometry components → scene components.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::world_serialization::WorldAssetRoot;
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::stairs::rough_stonework::TREAD;
use crate::assets::AssetPath;
use crate::placed::Placed;
use crate::scene_children;
use crate::stairs::geometry::Stair;
use crate::stairs::geometry_components::StairComponent;
use crate::stairs::{RoughStoneSpiralStair, RoughStoneStraightStair, WoodStraightStair};

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

pub fn rough_stone_stair(placed: &Placed<Stair>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw, piece.scale);
			match piece.geom {
				StairComponent::Tread => Box::new(posed_glb(TREAD, transform)) as Box<dyn Scene>,
				StairComponent::Spiral => Box::new(with_pose(
					transform,
					RoughStoneSpiralStair::from(piece.geom).scene_with_lod(lod_ref),
				)) as Box<dyn Scene>,
				StairComponent::Straight => Box::new(with_pose(
					transform,
					RoughStoneStraightStair::from(piece.geom).scene_with_lod(lod_ref),
				)) as Box<dyn Scene>,
			}
		})
		.collect();
	scene_children(children)
}

pub fn wood_stair(placed: &Placed<Stair>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw, piece.scale);
			let child: Box<dyn Scene> = match piece.geom {
				StairComponent::Tread | StairComponent::Spiral => {
					Box::new(RoughStoneSpiralStair::from(piece.geom).scene_with_lod(lod_ref))
				}
				StairComponent::Straight => {
					Box::new(WoodStraightStair::from(piece.geom).scene_with_lod(lod_ref))
				}
			};
			Box::new(with_pose(transform, child)) as Box<dyn Scene>
		})
		.collect();
	scene_children(children)
}
