//! Floor geometry components → scene components.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::world_serialization::WorldAssetRoot;
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::assets::floors::rough_stonework::CIRCLE_INSCRIBED_SQUARE;
use crate::assets::partitions::rough_stonework::LINEAR;
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

/// Map the vertical linear partition kit onto a **centered** unit floor rectangle.
///
/// Linear kit: \(X \in [-1, 1]\), \(Y \in [0, 1]\), \(Z \in [-0.2, 0.2]\).
/// Center \(Y\) onto the origin, pitch so wall height becomes \(+Z\), then scale to a
/// unit square in \(XZ\) with thickness along \(Y\).
fn rectangle_pose_from_linear_partition(placed: Transform) -> Transform {
	let center_kit = Transform::from_translation(Vec3::new(0.0, -0.5, 0.0));
	let wall_to_unit_floor =
		Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2))
			.with_scale(Vec3::new(0.5, 1.0 / 0.4, 1.0));
	placed.mul_transform(wall_to_unit_floor.mul_transform(center_kit))
}

pub fn rough_stone_floor(placed: &Placed<Floor>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw, piece.scale);
			match piece.geom {
				FloorComponent::Rectangle => {
					Box::new(posed_glb(LINEAR, rectangle_pose_from_linear_partition(transform)))
						as Box<dyn Scene>
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
