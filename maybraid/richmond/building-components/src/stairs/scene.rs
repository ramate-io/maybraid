//! Stair geometry components → scene components.

use bevy::prelude::{Children, Transform};
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::{Quat, Vec3};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::placed::Placed;
use crate::stairs::geometry::Stair;
use crate::stairs::geometry_components::StairComponent;
use crate::stairs::{RoughStoneSpiralStair, RoughStoneStraightStair, WoodStraightStair};

fn pose(translation: Vec3, yaw: f32) -> Transform {
	Transform::from_translation(translation).with_rotation(Quat::from_rotation_y(yaw))
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	bsn! {
		template_value(transform)
		Children [ ({child}) ]
	}
}

pub fn rough_stone_stair(placed: &Placed<Stair>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw);
			let child: Box<dyn Scene> = match piece.geom {
				StairComponent::Spiral => {
					Box::new(RoughStoneSpiralStair::from(piece.geom).scene_with_lod(lod_ref))
				}
				StairComponent::Straight => {
					Box::new(RoughStoneStraightStair::from(piece.geom).scene_with_lod(lod_ref))
				}
			};
			Box::new(with_pose(transform, child)) as Box<dyn Scene>
		})
		.collect();
	bsn! {
		Children [ {children} ]
	}
}

pub fn wood_stair(placed: &Placed<Stair>, lod_ref: &LodRef) -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = placed
		.into_geometry_components()
		.into_iter()
		.map(|piece| {
			let transform = pose(piece.translation, piece.yaw);
			let child: Box<dyn Scene> = match piece.geom {
				StairComponent::Spiral => {
					Box::new(RoughStoneSpiralStair::from(piece.geom).scene_with_lod(lod_ref))
				}
				StairComponent::Straight => {
					Box::new(WoodStraightStair::from(piece.geom).scene_with_lod(lod_ref))
				}
			};
			Box::new(with_pose(transform, child)) as Box<dyn Scene>
		})
		.collect();
	bsn! {
		Children [ {children} ]
	}
}
