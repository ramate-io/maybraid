//! [`BuildingComponents`] presentation for [`PanelComplex`].

use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use super::types::PanelComplex;

impl BuildingComponents for PanelComplex {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for panel in self.triangle_panels() {
			out.extend(panel.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<JointNode> {
		Layers::from_free(self.joint_nodes())
	}
}
