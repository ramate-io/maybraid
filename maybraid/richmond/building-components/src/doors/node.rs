//! Door IR node: style + geometry + placement.

use bevy::prelude::Transform;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy_math::Quat;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::doors::geometry::DoorGeometry;
use crate::doors::style::DoorStyle;
use crate::doors::tessellate::DoorKit;
use crate::doors::WoodDoorLeaf;
use crate::partitions::node::wall_kit_scene;
use crate::placed::Placement;
use crate::scene_children;

/// Authoring IR for a door feature.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorNode {
	pub style: DoorStyle,
	pub geometry: DoorGeometry,
	pub placement: Placement,
}

impl DoorNode {
	pub fn new(style: DoorStyle, geometry: DoorGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
		}
	}

	pub fn wood(geometry: DoorGeometry, placement: Placement) -> Self {
		Self::new(DoorStyle::Wood, geometry, placement)
	}
}

fn pose(placement: Placement) -> Transform {
	Transform::from_translation(placement.translation)
		.with_rotation(Quat::from_rotation_y(placement.yaw))
		.with_scale(placement.scale)
}

fn with_pose(transform: Transform, child: impl Scene + 'static) -> impl Scene + 'static {
	(
		child,
		bsn! {
			template_value(transform)
		},
	)
}

impl LodScene for DoorNode {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				let child: Box<dyn Scene> = match piece.geom {
					DoorKit::Leaf => Box::new(WoodDoorLeaf.scene_with_lod(lod_ref)),
					DoorKit::FramePiece(wall) => wall_kit_scene(wall, lod_ref),
				};
				Box::new(with_pose(transform, child)) as Box<dyn Scene>
			})
			.collect();
		scene_children(children)
	}
}
