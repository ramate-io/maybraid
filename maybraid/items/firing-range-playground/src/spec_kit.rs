//! Gallery-style kit: mesh slots plus the rolled surface look.

use crozon_character_items::{FirearmSpec, SlotLook};
use firearms::{FirearmComponents, FirearmKit, Layers, PartNode};
use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;

use crate::loadout::kit_from_spec;

/// Held firearm that paints catalog looks onto the assembled kit.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct RolledFirearm {
	pub spec: FirearmSpec,
	pub kit: FirearmKit,
}

impl RolledFirearm {
	pub fn from_spec(spec: FirearmSpec) -> Self {
		Self { spec, kit: kit_from_spec(spec) }
	}

	fn look_material(look: SlotLook) -> MaterialRef {
		MaterialRef::named(look.material.recipe_id()).with_palette([look.color.color()])
	}
}

fn paint(layers: Layers<PartNode>, look: SlotLook) -> Layers<PartNode> {
	let material = RolledFirearm::look_material(look);
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

impl FirearmComponents for RolledFirearm {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<firearms::RigNode> {
		self.kit.rig_nodes_for_level(level)
	}

	fn body_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		paint(self.kit.body_nodes_for_level(level), self.spec.looks.body)
	}

	fn barrel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		paint(self.kit.barrel_nodes_for_level(level), self.spec.looks.barrel)
	}

	fn trigger_box_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		paint(self.kit.trigger_box_nodes_for_level(level), self.spec.looks.trigger_box)
	}

	fn grip_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		paint(self.kit.grip_nodes_for_level(level), self.spec.looks.grip)
	}

	fn stock_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		paint(self.kit.stock_nodes_for_level(level), self.spec.looks.stock)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crozon_character_items::{FirearmMaterial, FirearmMesh, ItemColor};

	#[test]
	fn paints_the_body_look_onto_the_kit_node() {
		let mut spec = FirearmSpec::from_mesh(FirearmMesh::Bullpup);
		spec.looks.body =
			crozon_character_items::SlotLook::new(FirearmMaterial::LavaVeins, ItemColor::Red);
		let rolled = RolledFirearm::from_spec(spec);
		let nodes = rolled.body_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 1);
		assert_eq!(nodes[0].material, RolledFirearm::look_material(spec.looks.body));
	}
}
