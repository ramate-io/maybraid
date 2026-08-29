//! Flatten stick / foliage IR into [`VisualInstance`]s for packed visual LOD.
//!
//! One instance per IR placement. Kit collections emit one instance per member.
//! Empty finer slots on the *same* pose alias the coarsest authored [`SceneRef`]
//! so coarser-only instances still draw in any finer selected band.

use std::collections::HashMap;

use lod::gen::LodSceneLevel;
use lod::{Banded, NamedVisualLevel, VisualInstance};
use material_ref::{MaterialRef, MaterialRefKey};
use scene_ref::{SceneRef, TransformKey};

use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::node::FoliageNode;
use crate::scene_children::pose;
use crate::sticks::node::StickNode;
use crate::VegetationComponents;

/// Domain hook for contributing one or more vegetation sources to one packed
/// banded instance list.
///
/// This is deliberately separate from [`VegetationComponents`]: direct
/// vegetation can add itself, while composite domains can add nested,
/// independently placed values without a blanket implementation blocking their
/// custom behavior.
pub trait VegetationVisualPack {
	fn pack_vegetation_visual(&self, packer: &mut VegetationVisualPacker);
}

/// Accumulates independently authored vegetation sources before per-pose band
/// aliasing.
#[derive(Default)]
pub struct VegetationVisualPacker {
	by_pose: HashMap<(MaterialRefKey, TransformKey), Banded<Option<SceneRef>>>,
	materials: HashMap<(MaterialRefKey, TransformKey), MaterialRef>,
}

impl VegetationVisualPacker {
	pub fn new() -> Self {
		Self::default()
	}

	/// Add all four authored levels from one vegetation source.
	pub fn add_vegetation(&mut self, vegetation: &impl VegetationComponents) {
		for (band, level) in [
			(NamedVisualLevel::UltraLow, LodSceneLevel::UltraLow),
			(NamedVisualLevel::Low, LodSceneLevel::Low),
			(NamedVisualLevel::Medium, LodSceneLevel::Medium),
			(NamedVisualLevel::High, LodSceneLevel::High),
		] {
			self.add_level(vegetation, band, level);
		}
	}

	/// Add `source_level` placements to one visual `band`.
	///
	/// Keeping source and target explicit preserves authored cases such as a
	/// reduced-density Medium band built from High geometry.
	pub fn add_level(
		&mut self,
		vegetation: &impl VegetationComponents,
		band: NamedVisualLevel,
		source_level: LodSceneLevel,
	) {
		for (scene, material, transform) in pack_level_placements(vegetation, source_level) {
			let key = (MaterialRefKey::from(&material), TransformKey::new(transform));
			self.materials.entry(key.clone()).or_insert(material);
			let slot = self.by_pose.entry(key).or_default();
			*slot.for_level_mut(band) = Some(scene);
		}
	}

	/// Finish the list, aliasing empty finer slots on each independent pose.
	pub fn finish_aliased(mut self) -> PackedVegetationBands {
		for scenes in self.by_pose.values_mut() {
			alias_empty_finer_bands(scenes);
		}

		let instances = self
			.by_pose
			.into_iter()
			.filter_map(|(key, scenes)| {
				let material = self.materials.remove(&key)?;
				let (_material_key, transform_key) = key;
				let pose = transform_key.0;
				Some(VisualInstance {
					scenes,
					material,
					transform: bevy::math::Affine3A::from_scale_rotation_translation(
						pose.scale,
						pose.rotation,
						pose.translation,
					),
				})
			})
			.collect();
		PackedVegetationBands { instances }
	}
}

/// Packed UltraLow / Low / Medium / High placements, folded across bands when the
/// material and pose match.
#[derive(Debug, Clone, PartialEq)]
pub struct PackedVegetationBands {
	pub instances: Vec<VisualInstance>,
}

impl PackedVegetationBands {
	/// Filled slot counts after per-instance alias.
	pub fn band_slot_counts(&self) -> Banded<usize> {
		let mut counts = Banded::default();
		for instance in &self.instances {
			if instance.scene_for(NamedVisualLevel::UltraLow).is_some() {
				counts.ultra_low += 1;
			}
			if instance.scene_for(NamedVisualLevel::Low).is_some() {
				counts.low += 1;
			}
			if instance.scene_for(NamedVisualLevel::Medium).is_some() {
				counts.medium += 1;
			}
			if instance.scene_for(NamedVisualLevel::High).is_some() {
				counts.high += 1;
			}
		}
		counts
	}

	/// Instances that draw when `band` is selected (finest authored ≤ band).
	pub fn resolved_count(&self, band: NamedVisualLevel) -> usize {
		self.instances
			.iter()
			.filter(|instance| instance.scene_at_or_coarser(band).is_some())
			.count()
	}
}

/// Pack all visual bands coarse-to-fine. Empty finer slots alias per instance.
pub fn pack_vegetation_visual_aliased(
	vegetation: &impl VegetationVisualPack,
) -> PackedVegetationBands {
	let mut packer = VegetationVisualPacker::new();
	vegetation.pack_vegetation_visual(&mut packer);
	packer.finish_aliased()
}

fn alias_empty_finer_bands(scenes: &mut Banded<Option<SceneRef>>) {
	if scenes.low.is_none() {
		scenes.low.clone_from(&scenes.ultra_low);
	}
	if scenes.medium.is_none() {
		scenes.medium.clone_from(&scenes.low);
	}
	if scenes.high.is_none() {
		scenes.high.clone_from(&scenes.medium);
	}
}

/// Pack every stick / foliage placement at `level` (no cross-band alias).
pub fn pack_vegetation_visual(
	vegetation: &impl VegetationComponents,
	level: LodSceneLevel,
) -> Vec<VisualInstance> {
	let named = NamedVisualLevel::from_scene_level(level);
	pack_level_placements(vegetation, level)
		.into_iter()
		.map(|(scene, material, transform)| {
			let mut instance = VisualInstance::new(
				material,
				bevy::math::Affine3A::from_scale_rotation_translation(
					transform.scale,
					transform.rotation,
					transform.translation,
				),
			);
			if let Some(named) = named {
				*instance.scenes.for_level_mut(named) = Some(scene);
			}
			instance
		})
		.collect()
}

fn pack_level_placements(
	vegetation: &impl VegetationComponents,
	level: LodSceneLevel,
) -> Vec<(SceneRef, MaterialRef, bevy::prelude::Transform)> {
	let mut out = Vec::new();
	let stick_level = match level {
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
			LodSceneLevel::Low
		}
		other => other,
	};
	for node in vegetation.stick_nodes_for_level(stick_level).flatten() {
		pack_stick_node(&node, stick_level, &mut out);
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		pack_foliage_node(&node, level, &mut out);
	}
	out
}

fn pack_stick_node(
	node: &StickNode,
	level: LodSceneLevel,
	out: &mut Vec<(SceneRef, MaterialRef, bevy::prelude::Transform)>,
) {
	if matches!(
		level,
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_)
	) {
		return;
	}
	if let Some(collection) = &node.collection {
		for member in collection.members_for_level(level) {
			let Some(asset) = member.geometry.standard_glb_for_level(level) else {
				continue;
			};
			let placed = node.placement.compose_child(member.placement);
			out.push((asset.scene_ref(), node.material.clone(), pose(placed)));
		}
	} else if let Some(asset) = node.geometry.standard_glb_for_level(level) {
		out.push((asset.scene_ref(), node.material.clone(), pose(node.placement)));
	}
}

fn pack_foliage_node(
	node: &FoliageNode,
	level: LodSceneLevel,
	out: &mut Vec<(SceneRef, MaterialRef, bevy::prelude::Transform)>,
) {
	match &node.geometry {
		FoliageGeometry::CheapBall | FoliageGeometry::LayeredBall => {
			let asset = if matches!(node.geometry, FoliageGeometry::LayeredBall) {
				FoliageGeometry::layered_ball_glb_for_level(level)
			} else {
				FoliageGeometry::cheap_ball_glb_for_level(level)
			};
			out.push((asset.scene_ref(), node.material.clone(), pose(node.placement)));
		}
		FoliageGeometry::StraightFrond => {
			let asset = FoliageGeometry::straight_frond_glb_for_level(level);
			out.push((asset.scene_ref(), node.material.clone(), pose(node.placement)));
		}
		FoliageGeometry::StraightFrondSegment => {
			let asset = FoliageGeometry::straight_frond_segment_glb_for_level(level);
			out.push((asset.scene_ref(), node.material.clone(), pose(node.placement)));
		}
		FoliageGeometry::FrondCollection(collection) => {
			for member in collection.members_for_level(level) {
				let asset = FoliageGeometry::frond_kit_glb_for_level(member.kit, level);
				let placed = node.placement.compose_child(member.placement);
				out.push((asset.scene_ref(), node.material.clone(), pose(placed)));
			}
		}
		FoliageGeometry::CheapBallCollection(collection) => {
			let asset = FoliageGeometry::cheap_ball_glb_for_level(level);
			for placement in collection.placements_for_level(level) {
				let placed = node.placement.compose_child(placement);
				out.push((asset.scene_ref(), node.material.clone(), pose(placed)));
			}
		}
	}
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
	fn pack_ultralow_sticks_uses_visible_low_proxy() {
		let trunk = StickMember::trunk(Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)));
		let node = StickNode::collection(
			StickCollection::new([trunk]).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		let ultra = pack_vegetation_visual(&OneTrunk(node.clone()), LodSceneLevel::UltraLow);
		let low = pack_vegetation_visual(&OneTrunk(node), LodSceneLevel::Low);
		assert_eq!(ultra.len(), 1);
		assert!(ultra[0].scene_for(NamedVisualLevel::UltraLow).is_some());
		assert_eq!(low.len(), 1);
		assert!(low[0].scene_for(NamedVisualLevel::Low).is_some());
		assert!(low[0].scene_for(NamedVisualLevel::UltraLow).is_none());
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

	impl VegetationVisualPack for LowOnly {
		fn pack_vegetation_visual(&self, packer: &mut VegetationVisualPacker) {
			packer.add_vegetation(self);
		}
	}

	#[test]
	fn low_proxy_stick_remains_visible_through_finer_bands() {
		let trunk = StickMember::trunk(Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)));
		let node = StickNode::collection(
			StickCollection::new([trunk]).bake_bounds_from_members(),
			Placement::IDENTITY,
		);
		let bands = pack_vegetation_visual_aliased(&LowOnly(node));
		assert_eq!(bands.instances.len(), 1);
		let instance = &bands.instances[0];
		assert_eq!(
			instance.scene_for(NamedVisualLevel::UltraLow),
			instance.scene_for(NamedVisualLevel::Low)
		);
		assert_eq!(
			instance.scene_for(NamedVisualLevel::Low),
			instance.scene_for(NamedVisualLevel::Medium)
		);
		assert_eq!(
			instance.scene_for(NamedVisualLevel::Medium),
			instance.scene_for(NamedVisualLevel::High)
		);
		assert!(instance.scene_for(NamedVisualLevel::Low).is_some());
	}

	struct TwoSources {
		left: OneTrunk,
		right: OneTrunk,
	}

	impl VegetationVisualPack for TwoSources {
		fn pack_vegetation_visual(&self, packer: &mut VegetationVisualPacker) {
			packer.add_level(&self.left, NamedVisualLevel::High, LodSceneLevel::High);
			packer.add_level(&self.right, NamedVisualLevel::High, LodSceneLevel::High);
		}
	}

	#[test]
	fn multiple_sources_keep_independent_transforms() {
		let trunk = |x| {
			OneTrunk(StickNode::collection(
				StickCollection::new([StickMember::trunk(
					Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)),
				)])
				.bake_bounds_from_members(),
				Placement::new(Vec3::X * x, 0.0),
			))
		};
		let bands =
			pack_vegetation_visual_aliased(&TwoSources { left: trunk(-3.0), right: trunk(5.0) });
		assert_eq!(bands.instances.len(), 2);
		let mut xs: Vec<_> = bands
			.instances
			.iter()
			.map(|instance| instance.transform.translation.x)
			.collect();
		xs.sort_by(f32::total_cmp);
		assert_eq!(xs, vec![-3.0, 5.0]);
	}

	struct WoodyGrove {
		bin: FoliageNode,
		trunk: StickNode,
	}

	impl VegetationComponents for WoodyGrove {
		fn stick_nodes_for_level(&self, level: LodSceneLevel) -> crate::Layers<StickNode> {
			if matches!(level, LodSceneLevel::Low) {
				crate::Layers::from_free(vec![self.trunk.clone()])
			} else {
				crate::Layers::new()
			}
		}

		fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> crate::Layers<FoliageNode> {
			if matches!(level, LodSceneLevel::UltraLow) {
				crate::Layers::from_free(vec![self.bin.clone()])
			} else {
				crate::Layers::new()
			}
		}
	}

	impl VegetationVisualPack for WoodyGrove {
		fn pack_vegetation_visual(&self, packer: &mut VegetationVisualPacker) {
			packer.add_vegetation(self);
		}
	}

	#[test]
	fn aliased_ultralow_bins_draw_when_low_exists() {
		let trunk = StickMember::trunk(Placement::IDENTITY.with_scale(Vec3::new(0.4, 4.0, 0.4)));
		let grove = WoodyGrove {
			bin: FoliageNode::cheap_ball(Placement::new(Vec3::X * 8.0, 0.0)),
			trunk: StickNode::collection(
				StickCollection::new([trunk]).bake_bounds_from_members(),
				Placement::IDENTITY,
			),
		};
		let low_only = pack_vegetation_visual(&grove, LodSceneLevel::Low);
		let ultra_only = pack_vegetation_visual(&grove, LodSceneLevel::UltraLow);
		let bands = pack_vegetation_visual_aliased(&grove);
		assert_eq!(low_only.len(), 1);
		assert_eq!(ultra_only.len(), 2);
		assert_eq!(bands.instances.len(), 2);
		assert_eq!(bands.band_slot_counts().low, 2);
		assert_eq!(bands.resolved_count(NamedVisualLevel::Low), 2);
		assert_eq!(bands.resolved_count(NamedVisualLevel::UltraLow), 2);
		assert!(bands
			.instances
			.iter()
			.all(|instance| { instance.scene_at_or_coarser(NamedVisualLevel::Low).is_some() }));
	}
}
