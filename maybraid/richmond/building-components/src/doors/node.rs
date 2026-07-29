//! Door IR node: style + geometry + placement.

use bevy::scene::prelude::Scene;
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::doors::geometry::DoorGeometry;
use crate::doors::style::DoorStyle;
use crate::doors::tessellate::DoorKit;
use crate::doors::WoodDoorLeaf;
use crate::partitions::node::partition_kit_scene;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children, with_pose};

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

impl LodScene for DoorNode {
	fn scene_lod_status(
		&self,
		_lod_ref: &LodRef,
	) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

		fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: lod::gen::LodSceneLevel,
	) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				let child: Box<dyn Scene> = match piece.geom {
					DoorKit::Leaf => Box::new(WoodDoorLeaf.scene_with_lod(lod_ref)),
					DoorKit::FramePiece(kit) => partition_kit_scene(kit, lod_ref),
				};
				Box::new(with_pose(transform, child)) as Box<dyn Scene>
			})
			.collect();
		scene_children(children)
	}
}
