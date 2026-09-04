//! Inventory firearm realization shared by generated mob characters.

use crozon_character_items::{
	FirearmBarrel, FirearmGrip, FirearmKitSpec, FirearmMesh, FirearmSpec, FirearmStock,
	FirearmTriggerBox, SlotLook,
};
use firearms::{
	BarrelMesh, BodyMesh, FirearmComponents, FirearmKit, GripMesh, Layers, PartNode, RigNode,
	StockMesh, TriggerBoxMesh,
};
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeneratedFirearm {
	pub spec: FirearmSpec,
	pub kit: FirearmKit,
}

impl GeneratedFirearm {
	pub fn from_spec(spec: FirearmSpec) -> Self {
		Self { spec, kit: kit_from_spec(spec.kit) }
	}

	fn look_material(look: SlotLook) -> MaterialRef {
		MaterialRef::named(look.material.recipe_id()).with_palette([look.color.color()])
	}

	fn paint(layers: Layers<PartNode>, look: SlotLook) -> Layers<PartNode> {
		let material = Self::look_material(look);
		let mut out = Layers::new();
		out.extend_free(layers.free.into_iter().map(|node| node.with_material(material.clone())));
		for (layer, nodes) in layers.labeled {
			out.extend_labeled(
				layer,
				nodes.into_iter().map(|node| node.with_material(material.clone())),
			);
		}
		out
	}
}

fn kit_from_spec(spec: FirearmKitSpec) -> FirearmKit {
	FirearmKit {
		body: match spec.body {
			FirearmMesh::Bullpup => BodyMesh::Bullpup,
			FirearmMesh::Silopup => BodyMesh::Silopup,
			FirearmMesh::Reltor => BodyMesh::Reltor,
			FirearmMesh::Samsonist => BodyMesh::Samsonist,
			FirearmMesh::Snailer => BodyMesh::Snailer,
		},
		barrel: match spec.barrel {
			FirearmBarrel::None => BarrelMesh::None,
			FirearmBarrel::Bullpup => BarrelMesh::Bullpup,
			FirearmBarrel::Laznard => BarrelMesh::Laznard,
		},
		trigger_box: match spec.trigger_box {
			FirearmTriggerBox::None => TriggerBoxMesh::None,
			FirearmTriggerBox::Keelripe => TriggerBoxMesh::Keelripe,
			FirearmTriggerBox::Paddle => TriggerBoxMesh::Paddle,
			FirearmTriggerBox::Reltor => TriggerBoxMesh::Reltor,
		},
		grip: match spec.grip {
			FirearmGrip::None => GripMesh::None,
			FirearmGrip::BumpHandle => GripMesh::BumpHandle,
		},
		stock: match spec.stock {
			FirearmStock::None => StockMesh::None,
		},
	}
}

impl FirearmComponents for GeneratedFirearm {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.kit.rig_nodes_for_level(level)
	}

	fn body_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.body_nodes_for_level(level), self.spec.looks.body)
	}

	fn barrel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.barrel_nodes_for_level(level), self.spec.looks.barrel)
	}

	fn trigger_box_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.trigger_box_nodes_for_level(level), self.spec.looks.trigger_box)
	}

	fn grip_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.grip_nodes_for_level(level), self.spec.looks.grip)
	}

	fn stock_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		Self::paint(self.kit.stock_nodes_for_level(level), self.spec.looks.stock)
	}
}
