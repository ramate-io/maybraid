//! Mix-and-match firearm: required body plus optional kit slots.

use firearms_components::assets::guns;
use firearms_components::{FirearmComponents, Layers, PartNode, RigNode};
use lod::gen::LodSceneLevel;

use crate::parts::{BarrelMesh, BodyMesh, GripMesh, StockMesh, TriggerBoxMesh};

/// Assembled firearm: receiver + body, and whatever other slots are filled.
///
/// Barrel, trigger box, grip, and stock may be [`None`](BarrelMesh::None). Body
/// is always present.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, bevy::prelude::Component)]
pub struct FirearmKit {
	pub body: BodyMesh,
	pub barrel: BarrelMesh,
	pub trigger_box: TriggerBoxMesh,
	pub grip: GripMesh,
	pub stock: StockMesh,
}

impl FirearmKit {
	pub fn body(body: BodyMesh) -> Self {
		Self { body, ..Self::default() }
	}

	pub fn label(self) -> String {
		format!(
			"body={} barrel={} trigger-box={} grip={} stock={}",
			self.body.label(),
			self.barrel.label(),
			self.trigger_box.label(),
			self.grip.label(),
			self.stock.label(),
		)
	}
}

fn receiver_rig() -> RigNode {
	RigNode::receiver("firearm-rig", guns::FIREARM_RIG.as_str())
}

fn optional_layer(name: &'static str, node: Option<PartNode>) -> Layers<PartNode> {
	match node {
		Some(node) => Layers::from_labeled(name, vec![node]),
		None => Layers::new(),
	}
}

impl FirearmComponents for FirearmKit {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::from_labeled("receiver", vec![receiver_rig()])
	}

	fn body_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::from_labeled("body", vec![self.body.node()])
	}

	fn barrel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("barrel", self.barrel.node())
	}

	fn trigger_box_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("trigger_box", self.trigger_box.node())
	}

	fn grip_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("grip", self.grip.node())
	}

	fn stock_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		optional_layer("stock", self.stock.node())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use firearms_components::{FirearmPartSlot, SocketRef};

	#[test]
	fn default_kit_is_body_only() {
		let kit = FirearmKit::default();
		assert_eq!(kit.body, BodyMesh::Bullpup);
		assert_eq!(kit.body_nodes_for_level(LodSceneLevel::High).len(), 1);
		assert!(kit.barrel_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(kit.trigger_box_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(kit.grip_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(kit.stock_nodes_for_level(LodSceneLevel::High).is_empty());
	}

	#[test]
	fn optional_barrel_sockets_when_set() {
		let kit = FirearmKit { barrel: BarrelMesh::Laznard, ..FirearmKit::body(BodyMesh::Silopup) };
		let bodies = kit.body_nodes_for_level(LodSceneLevel::High).flatten();
		let barrels = kit.barrel_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(bodies[0].slot, FirearmPartSlot::Body);
		assert_eq!(bodies[0].socket, Some(SocketRef::bone("body")));
		assert_eq!(barrels[0].slot, FirearmPartSlot::Barrel);
		assert_eq!(barrels[0].socket, Some(SocketRef::bone("barrel")));
		assert_eq!(barrels[0].scene.path, guns::LAZNARD_BARREL.as_str());
	}
}
