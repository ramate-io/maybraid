//! Door geometry components → scene components.

use bevy::prelude::{Children, Transform};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::doors::geometry::Door;
use crate::doors::geometry_components::DoorComponent;
use crate::doors::WoodDoorLeaf;
use crate::partitions::scene::wall_component_scene;
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

pub fn door_scene(placed: &Placed<Door>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw);
			let child: Box<dyn Scene> = match piece.geom {
				DoorComponent::Leaf => Box::new(WoodDoorLeaf::from(piece.geom).scene_with_lod(lod_ref)),
				DoorComponent::FramePiece(wall) => wall_component_scene(wall, lod_ref),
			};
			Box::new(with_pose(transform, child)) as Box<dyn Scene>
		})
		.collect();
	bsn! {
		Children [ {children} ]
	}
}
