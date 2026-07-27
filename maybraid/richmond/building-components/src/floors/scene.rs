//! Floor geometry components → scene components.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::world_serialization::WorldAssetRoot;
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::floors::rough_stonework::{CIRCLE_INSCRIBED_SQUARE, RECTANGLE};
use crate::assets::AssetPath;
use crate::floors::geometry::Floor;
use crate::floors::geometry_components::FloorComponent;
use crate::floors::{
	RoughStoneFloorArcFill, RoughStoneFloorStructFill, WoodFloorArcFill, WoodFloorRectangle,
	WoodFloorStructFill,
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

pub fn rough_stone_floor(placed: &Placed<Floor>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw, piece.scale);
			match piece.geom {
				FloorComponent::Rectangle => {
					Box::new(posed_glb(RECTANGLE, transform)) as Box<dyn Scene>
				}
				FloorComponent::CircleInscribedSquare => {
					Box::new(posed_glb(CIRCLE_INSCRIBED_SQUARE, transform)) as Box<dyn Scene>
				}
				FloorComponent::ArcFill(_) => Box::new(with_pose(
					transform,
					RoughStoneFloorArcFill::from(piece.geom).scene_with_lod(lod_ref),
				)) as Box<dyn Scene>,
				FloorComponent::StructFill => Box::new(with_pose(
					transform,
					RoughStoneFloorStructFill::from(piece.geom).scene_with_lod(lod_ref),
				)) as Box<dyn Scene>,
			}
		})
		.collect();
	scene_children(children)
}

pub fn wood_floor(placed: &Placed<Floor>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw, piece.scale);
			let child: Box<dyn Scene> = match piece.geom {
				FloorComponent::Rectangle | FloorComponent::CircleInscribedSquare => {
					Box::new(WoodFloorRectangle::from(piece.geom).scene_with_lod(lod_ref))
				}
				FloorComponent::ArcFill(_) => {
					Box::new(WoodFloorArcFill::from(piece.geom).scene_with_lod(lod_ref))
				}
				FloorComponent::StructFill => {
					Box::new(WoodFloorStructFill::from(piece.geom).scene_with_lod(lod_ref))
				}
			};
			Box::new(with_pose(transform, child)) as Box<dyn Scene>
		})
		.collect();
	scene_children(children)
}
