//! Flatten stick / foliage IR into [`MultiSceneMerge`] keys for packed visual LOD.
//!
//! One merge per material. Kit-local member poses are composed into the node
//! placement so a grove can intern UltraLow / Low / Medium without spawning
//! hosts.

use lod::gen::LodSceneLevel;
use material_ref::MaterialRef;
use scene_ref::{MultiSceneMerge, MultiScenePart};

use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::node::FoliageNode;
use crate::scene_children::pose;
use crate::sticks::node::StickNode;
use crate::VegetationComponents;

/// One internable merge and the deferred material that should tint it.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualPackPart {
	pub material: MaterialRef,
	pub merge: MultiSceneMerge,
}

/// UltraLow / Low / Medium packs. Empty finer bands alias the next coarser pack
/// so woody groves (empty Medium plant IR) share one cook key.
#[derive(Debug, Clone, PartialEq)]
pub struct PackedVegetationBands {
	pub ultra_low: Vec<VisualPackPart>,
	pub low: Vec<VisualPackPart>,
	pub medium: Vec<VisualPackPart>,
}

/// Pack UltraLow, then Low, then Medium, aliasing empty bands downward.
pub fn pack_vegetation_visual_aliased(
	vegetation: &impl VegetationComponents,
) -> PackedVegetationBands {
	let ultra_low = pack_vegetation_visual(vegetation, LodSceneLevel::UltraLow);
	let mut low = pack_vegetation_visual(vegetation, LodSceneLevel::Low);
	let mut medium = pack_vegetation_visual(vegetation, LodSceneLevel::Medium);
	if low.is_empty() {
		low.clone_from(&ultra_low);
	}
	if medium.is_empty() {
		medium.clone_from(&low);
	}
	PackedVegetationBands { ultra_low, low, medium }
}

/// Pack every stick / foliage node at `level`, folded by [`MaterialRef`].
pub fn pack_vegetation_visual(
	vegetation: &impl VegetationComponents,
	level: LodSceneLevel,
) -> Vec<VisualPackPart> {
	let mut parts = Vec::new();
	for node in vegetation.stick_nodes_for_level(level).flatten() {
		if let Some(part) = pack_stick_node(&node, level) {
			parts.push(part);
		}
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		if let Some(part) = pack_foliage_node(&node, level) {
			parts.push(part);
		}
	}
	fold_by_material(parts)
}

fn fold_by_material(parts: Vec<VisualPackPart>) -> Vec<VisualPackPart> {
	let mut out: Vec<VisualPackPart> = Vec::new();
	for part in parts {
		if let Some(existing) = out.iter_mut().find(|p| p.material == part.material) {
			existing.merge.parts.extend(part.merge.parts);
		} else {
			out.push(part);
		}
	}
	out.retain(|part| !part.merge.parts.is_empty());
	out
}

fn pack_stick_node(node: &StickNode, level: LodSceneLevel) -> Option<VisualPackPart> {
	if matches!(
		level,
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_)
	) {
		return None;
	}
	let mut parts = Vec::new();
	if let Some(collection) = &node.collection {
		for member in collection.members_for_level(level) {
			let asset = member.geometry.standard_glb_for_level(level)?;
			let placed = node.placement.compose_child(member.placement);
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(placed)));
		}
	} else {
		let asset = node.geometry.standard_glb_for_level(level)?;
		parts.push(MultiScenePart::new(asset.scene_ref(), pose(node.placement)));
	}
	if parts.is_empty() {
		return None;
	}
	Some(VisualPackPart { material: node.material.clone(), merge: MultiSceneMerge::new(parts) })
}

fn pack_foliage_node(node: &FoliageNode, level: LodSceneLevel) -> Option<VisualPackPart> {
	let mut parts = Vec::new();
	match &node.geometry {
		FoliageGeometry::CheapBall | FoliageGeometry::LayeredBall => {
			let asset = if matches!(node.geometry, FoliageGeometry::LayeredBall) {
				FoliageGeometry::layered_ball_glb_for_level(level)
			} else {
				FoliageGeometry::cheap_ball_glb_for_level(level)
			};
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(node.placement)));
		}
		FoliageGeometry::StraightFrond => {
			let asset = FoliageGeometry::straight_frond_glb_for_level(level);
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(node.placement)));
		}
		FoliageGeometry::StraightFrondSegment => {
			let asset = FoliageGeometry::straight_frond_segment_glb_for_level(level);
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(node.placement)));
		}
		FoliageGeometry::FrondCollection(collection) => {
			for member in collection.members_for_level(level) {
				let asset = FoliageGeometry::frond_kit_glb_for_level(member.kit, level);
				let placed = node.placement.compose_child(member.placement);
				parts.push(MultiScenePart::new(asset.scene_ref(), pose(placed)));
			}
		}
		FoliageGeometry::CheapBallCollection(collection) => {
			let asset = FoliageGeometry::cheap_ball_glb_for_level(level);
			for placement in collection.placements_for_level(level) {
				let placed = node.placement.compose_child(placement);
				parts.push(MultiScenePart::new(asset.scene_ref(), pose(placed)));
			}
		}
	}
	if parts.is_empty() {
		return None;
	}
	Some(VisualPackPart { material: node.material.clone(), merge: MultiSceneMerge::new(parts) })
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::placed::Placement;
	use crate::sticks::{StickCollection, StickMember};
	use bevy::prelude::Vec3;

	struct OneTrunk(StickNode);

	impl VegetationComponents for OneTrunk {
		fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> crate::Layers<StickNode> {
			crate::Layers::from_free(vec![self.0.clone()])
		}
	}

	#[test]
	fn pack_skips_ultralow_sticks() {
		let trunk = StickMember::trunk(Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)));
		let node = StickNode::collection(
			StickCollection::new([trunk]).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		assert!(pack_vegetation_visual(&OneTrunk(node.clone()), LodSceneLevel::UltraLow).is_empty());
		let low = pack_vegetation_visual(&OneTrunk(node), LodSceneLevel::Low);
		assert_eq!(low.len(), 1);
		assert!(!low[0].merge.parts.is_empty());
	}

	struct LowOnly(StickNode);

	impl VegetationComponents for LowOnly {
		fn stick_nodes_for_level(&self, level: LodSceneLevel) -> crate::Layers<StickNode> {
			if matches!(level, LodSceneLevel::Low) {
				crate::Layers::from_free(vec![self.0.clone()])
			} else {
				crate::Layers::new()
			}
		}
	}

	#[test]
	fn aliased_empty_medium_shares_low() {
		let trunk = StickMember::trunk(Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)));
		let node = StickNode::collection(
			StickCollection::new([trunk]).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		let bands = pack_vegetation_visual_aliased(&LowOnly(node));
		assert!(bands.ultra_low.is_empty());
		assert_eq!(bands.low, bands.medium);
		assert!(!bands.low.is_empty());
	}
}
