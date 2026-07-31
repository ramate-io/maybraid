//! [`BuildingComponents`] presentation for [`PanelComplex`].

use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::BuildingComponents;

use super::types::PanelComplex;

impl BuildingComponents for PanelComplex {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Vec<PanelNode> {
		let mut out = Vec::new();
		for panel in self.triangle_panels() {
			out.extend(panel.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Vec<JointNode> {
		self.joint_nodes()
	}
}
