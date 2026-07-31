//! Larger top-floor perch capping the Wizard's Tower.
//!
//! Same treatment as a regular storey for now: [`crate::arcs::PortalRingWall`] + n-gon floor.

use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::{LodScene, LodSceneLevel};
use lod::lod_ref::LodRef;
use procedural_common::NoiseParams;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::partitions::PartitionStyle;
use richmond_building_components::scene_children;
use richmond_building_components::{
	append_component_scenes, confined_scene, BuildingComponents, Layers, ParentConfines,
	PartitionNode,
};

use crate::arcs::{portal_ring_wall, PortalRingParams, PortalRingWall};
use crate::paneling::ApproximatedCircle;
use crate::wizards_tower::floor_fill::{circular_floor_with_spire_hole, SPIRE_HALF_FRAC};
use crate::wizards_tower::must_assign_cardinal_portals;
use crate::CellConstraints;

/// Top perch: wider circular platform over the column.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerPerch {
	pub constraints: CellConstraints,
	/// Storey height in meters (outer ring wall \(Y\) scale).
	pub storey_height: f32,
	pub ring_wall: PortalRingWall,
	/// N-gon floor annulus with concentric spire hole.
	pub floor_disk: ApproximatedCircle,
}

impl WizardsTowerPerch {
	/// Build from this perch's subsetted constraints, storey height, and portal noise.
	pub fn new(
		constraints: CellConstraints,
		storey_height: f32,
		portal_noise: NoiseParams,
	) -> Self {
		let storey_height = storey_height.max(1e-4);
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let spire_radius = SPIRE_HALF_FRAC * radius;
		let floor_disk = circular_floor_with_spire_hole(center_xz, radius, spire_radius);

		let ring_wall = portal_ring_wall(PortalRingParams {
			center_xz,
			radius,
			storey_height,
			arc_degrees: 360.0,
			must_assign: must_assign_cardinal_portals(),
			must_not_assign: vec![],
			portal_noise,
			optional_portals: (0, 2),
			style: PartitionStyle::RoughStonework,
		});

		Self {
			storey_height,
			ring_wall,
			floor_disk,
			constraints,
		}
	}

	pub(crate) fn emit_external_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		append_component_scenes(self, lod_ref, LodSceneLevel::Medium, children);
	}

	pub(crate) fn emit_internal_features(
		&self,
		children: &mut Vec<Box<dyn Scene>>,
		lod_ref: &LodRef,
	) {
		let confines =
			ParentConfines::internal(self.storey_confine_center(), self.storey_confine_radius());
		for node in self.panel_nodes_for_level(LodSceneLevel::High).flatten() {
			children.push(Box::new(confined_scene(
				confines,
				node.scene_with_lod(lod_ref),
			)));
		}
	}

	fn storey_confine_center(&self) -> Vec3 {
		let aabb = &self.constraints.aabb;
		Vec3::from((aabb.min + aabb.max) * 0.5)
	}

	fn storey_confine_radius(&self) -> f32 {
		let aabb = &self.constraints.aabb;
		let extent = aabb.max - aabb.min;
		(0.5 * extent.x.min(extent.z)).max(1e-4)
	}

	fn is_detail_level(level: LodSceneLevel) -> bool {
		matches!(level, LodSceneLevel::High)
	}

	fn is_structure_level(level: LodSceneLevel) -> bool {
		matches!(level, LodSceneLevel::High | LodSceneLevel::Medium)
	}
}

impl BuildingComponents for WizardsTowerPerch {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		if Self::is_structure_level(level) {
			self.ring_wall.sweep.partition_nodes_for_level(level)
		} else {
			Layers::new()
		}
	}

	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		if Self::is_detail_level(level) {
			self.floor_disk.panel_nodes_for_level(level)
		} else {
			Layers::new()
		}
	}
}

impl LodScene for WizardsTowerPerch {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> lod::gen::LodSceneStatus {
		lod::gen::LodSceneStatus::Unchanged
	}

	fn scene_with_level(
		&self,
		lod_ref: &LodRef,
		_level: LodSceneLevel,
	) -> impl Scene + 'static {
		let mut children: Vec<Box<dyn Scene>> = Vec::new();
		self.emit_external_features(&mut children, lod_ref);
		self.emit_internal_features(&mut children, lod_ref);
		scene_children(children)
	}
}
