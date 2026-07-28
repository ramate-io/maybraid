//! Roof geometry components → scene components.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::placed::Placed;
use crate::roofs::geometry::Roof;
use crate::roofs::geometry_components::RoofComponent;
use crate::roofs::{RoughStonePerchRoof, RoughStoneSpireRoof, WoodPerchDeck};
use crate::scene_children;

fn pose(translation: Vec3, yaw: f32, scale: Vec3) -> Transform {
	Transform::from_translation(translation)
		.with_rotation(Quat::from_rotation_y(yaw))
		.with_scale(scale)
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

pub fn roof_scene(placed: &Placed<Roof>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw, piece.scale);
			let child: Box<dyn Scene> = match piece.geom {
				RoofComponent::Spire => {
					Box::new(RoughStoneSpireRoof::from(piece.geom).scene_with_lod(lod_ref))
				}
				RoofComponent::Perch => {
					Box::new(RoughStonePerchRoof::from(piece.geom).scene_with_lod(lod_ref))
				}
				RoofComponent::Deck => Box::new(WoodPerchDeck::from(piece.geom).scene_with_lod(lod_ref)),
			};
			Box::new(with_pose(transform, child)) as Box<dyn Scene>
		})
		.collect();
	scene_children(children)
}
