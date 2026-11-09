//! Shared-kit instance lists for one capability-compatible woody tile / LOD band.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::cache::AdmissionReason;
use crate::stick_physics::PackedPlantRecipe;
use crate::ForestGroveTile;
use bevy::math::{Mat4, Vec3, Vec4};
use lod::{LodSceneLevel, VisualLodPrimitive, VisualMaterialKind};
use material_ref::{MaterialRef, MaterialRefKey};
use vegetation_components::{
	FoliageGeometry, FoliageNode, Placement, StickNode, LEAF_MATERIAL, STICK_MATERIAL,
};

/// One posed kit instance. Transform is grove-local (host transform is identity).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackedInstance {
	pub world_from_local: Mat4,
	pub plant_origin: Vec3,
	/// Collection root and child bake scale reproduce the existing merged-leaf
	/// shader's per-vertex centroid calculation, including nonuniform poses.
	pub collection_root: Mat4,
	pub part_scale: f32,
	pub merged_leaf: bool,
}

impl PackedInstance {
	fn plain(model: Mat4, origin: Vec3) -> Self {
		Self {
			world_from_local: model,
			plant_origin: origin,
			collection_root: model,
			part_scale: 1.0,
			merged_leaf: false,
		}
	}
}

/// Grouping key: one shared GLB + one resolved material recipe.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PackedBatchKey {
	pub kit: &'static str,
	pub material_kind: VisualMaterialKind,
	pub material_hash: u64,
}

/// Cached CPU batch. GPU bytes are uploaded once per revision.
#[derive(Clone, Debug)]
pub struct PackedBatch {
	pub key: PackedBatchKey,
	pub base_color: Vec4,
	pub instances: Arc<[PackedInstance]>,
}

impl PackedBatch {
	pub fn payload_bytes(&self) -> u64 {
		(self.instances.len() * std::mem::size_of::<PackedInstance>()) as u64
	}

	pub fn kit_id(&self) -> u64 {
		hash_bits(self.key.kit)
	}

	pub fn visual_primitive(&self, level: LodSceneLevel) -> VisualLodPrimitive {
		VisualLodPrimitive::packed_batch(
			level,
			self.kit_id(),
			self.key.material_kind,
			self.instances.len() as u32,
		)
	}
}

impl PackedBatchKey {
	pub fn new(kit: &'static str, material: &MaterialRef, kind: VisualMaterialKind) -> Self {
		Self { kit, material_kind: kind, material_hash: hash_material(material) }
	}
}

/// Assemble exactly the ordinary grove presentation policy for this band.
/// Detailed typed plants and grove proxy IR are intentionally distinct sources.
pub(crate) fn pack_grove_band(
	tile: &ForestGroveTile,
	plants: &[Arc<dyn PackedPlantRecipe>],
	level: LodSceneLevel,
	proxy_shadows: bool,
) -> Result<Vec<PackedBatch>, AdmissionReason> {
	let policy = tile.woody_lod().ok_or(AdmissionReason::UnsupportedAdapter)?;
	let mut groups = BTreeMap::new();
	let mut shadows = Vec::new();
	if let Some(plant_level) = policy.nest_plant_level(level) {
		for plant in plants {
			let sticks = plant.stick_nodes_for_level(plant_level).flatten();
			let foliage = plant.foliage_nodes_for_level(plant_level).flatten();
			validate_nodes(&sticks, &foliage, plant_level)?;
			for node in &sticks {
				push_stick(&mut groups, node, plant_level, plant.origin());
			}
			for node in &foliage {
				push_foliage(&mut groups, node, plant_level, plant.origin());
			}
			if let Some(lod) = plant.structural_lod() {
				let model = Mat4::from_scale_rotation_translation(
					Vec3::splat(lod.tree_radius.max(0.25)),
					bevy::math::Quat::IDENTITY,
					lod.center,
				);
				shadows.push(PackedInstance::plain(model, lod.center));
			}
		}
	} else {
		// Ordinary flattened_canopy_proxy_chunks uses Low sticks at UltraLow,
		// and merges canopy proxies *before* choosing their wind/centroid input.
		let stick_level = if level == LodSceneLevel::UltraLow { LodSceneLevel::Low } else { level };
		let sticks = tile.proxy_stick_nodes(stick_level);
		let foliage = FoliageNode::merge_canopy_proxies(tile.proxy_foliage_nodes(level));
		validate_nodes(&sticks, &foliage, stick_level)?;
		for node in &sticks {
			push_stick(&mut groups, node, stick_level, node.placement.translation);
		}
		for node in &foliage {
			push_foliage(&mut groups, node, level, node.placement.translation);
			match &node.geometry {
				FoliageGeometry::CheapBall => shadows.push(PackedInstance::plain(
					node.placement.affine(),
					node.placement.translation,
				)),
				FoliageGeometry::CheapBallCollection(collection) => {
					for child in collection.placements_for_level(level) {
						let pose = node.placement.compose_child(child);
						shadows.push(PackedInstance::plain(pose.affine(), pose.translation));
					}
				}
				_ => {}
			}
		}
	}
	let mut batches = finish_groups(groups);
	if proxy_shadows && !shadows.is_empty() {
		batches.push(PackedBatch {
			key: PackedBatchKey {
				kit: SHADOW_KIT,
				material_kind: VisualMaterialKind::ShadowProxy,
				material_hash: 0,
			},
			base_color: Vec4::ONE,
			instances: shadows.into(),
		});
	}
	Ok(batches)
}
pub const SHADOW_KIT: &str = "@canopy-shadow-proxy";

fn material_is(material: &MaterialRef, expected: &str) -> bool {
	matches!(&material.name, material_ref::MaterialId::Name(name) if name == expected)
		&& material.rasters.is_empty()
		&& material.scalars.is_empty()
}
fn validate_nodes(
	sticks: &[StickNode],
	foliage: &[FoliageNode],
	level: LodSceneLevel,
) -> Result<(), AdmissionReason> {
	for node in sticks {
		if !material_is(&node.material, STICK_MATERIAL) {
			return Err(AdmissionReason::UnsupportedMaterial);
		}
		if let Some(collection) = &node.collection {
			if collection
				.members_for_level(level)
				.iter()
				.any(|member| member.geometry.standard_glb_for_level(level).is_none())
			{
				return Err(AdmissionReason::UnsupportedGeometry);
			}
		} else if node.geometry.standard_glb_for_level(level).is_none() {
			return Err(AdmissionReason::UnsupportedGeometry);
		}
	}
	for node in foliage {
		if !matches!(
			node.geometry,
			FoliageGeometry::CheapBall
				| FoliageGeometry::CheapBallCollection(_)
				| FoliageGeometry::LayeredBall
		) {
			return Err(AdmissionReason::UnsupportedGeometry);
		}
		if !material_is(&node.material, LEAF_MATERIAL) {
			return Err(AdmissionReason::UnsupportedMaterial);
		}
	}
	Ok(())
}

fn push_stick(
	groups: &mut BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)>,
	node: &StickNode,
	level: LodSceneLevel,
	plant_origin: Vec3,
) {
	if let Some(collection) = &node.collection {
		for member in collection.members_for_level(level) {
			let Some(asset) = member.geometry.standard_glb_for_level(level) else {
				continue;
			};
			let key =
				PackedBatchKey::new(asset.as_str(), &node.material, VisualMaterialKind::Stick);
			groups
				.entry(key)
				.or_insert_with(|| {
					(base_color(&node.material, VisualMaterialKind::Stick), Vec::new())
				})
				.1
				.push(PackedInstance::plain(
					node.placement.affine() * member.placement.affine(),
					plant_origin,
				));
		}
		return;
	}
	let Some(asset) = node.geometry.standard_glb_for_level(level) else {
		return;
	};
	push_instance(
		groups,
		asset.as_str(),
		&node.material,
		VisualMaterialKind::Stick,
		node.placement,
		plant_origin,
	);
}

fn push_foliage(
	groups: &mut BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)>,
	node: &FoliageNode,
	level: LodSceneLevel,
	_plant_origin: Vec3,
) {
	let kind = material_kind(&node.material, &node.geometry);
	let plant_origin = node.placement.translation;
	match &node.geometry {
		FoliageGeometry::CheapBall => push_instance(
			groups,
			FoliageGeometry::cheap_ball_glb_for_level(level).as_str(),
			&node.material,
			kind,
			node.placement,
			plant_origin,
		),
		FoliageGeometry::LayeredBall => push_instance(
			groups,
			FoliageGeometry::layered_ball_glb_for_level(level).as_str(),
			&node.material,
			kind,
			node.placement,
			plant_origin,
		),
		FoliageGeometry::StraightFrond => push_instance(
			groups,
			FoliageGeometry::straight_frond_glb_for_level(level).as_str(),
			&node.material,
			kind,
			node.placement,
			plant_origin,
		),
		FoliageGeometry::StraightFrondSegment => push_instance(
			groups,
			FoliageGeometry::straight_frond_segment_glb_for_level(level).as_str(),
			&node.material,
			kind,
			node.placement,
			plant_origin,
		),
		FoliageGeometry::FrondCollection(collection) => {
			for member in collection.members_for_level(level) {
				push_instance(
					groups,
					FoliageGeometry::frond_kit_glb_for_level(member.kit, level).as_str(),
					&node.material,
					kind,
					node.placement.compose_child(member.placement),
					plant_origin,
				);
			}
		}
		FoliageGeometry::CheapBallCollection(collection) => {
			let kit = FoliageGeometry::cheap_ball_glb_for_level(level).as_str();
			for placement in collection.placements_for_level(level) {
				let composed = node.placement.compose_child(placement);
				let merged = matches!(
					node.collection_present,
					vegetation_components::CollectionPresent::Merge
				);
				let origin = if merged { plant_origin } else { composed.translation };
				let key = PackedBatchKey::new(kit, &node.material, kind);
				let instance = PackedInstance {
					world_from_local: if merged {
						node.placement.affine() * placement.affine()
					} else {
						composed.affine()
					},
					plant_origin: origin,
					collection_root: node.placement.affine(),
					part_scale: placement.scale.abs().max_element().max(1e-4),
					merged_leaf: merged,
				};
				groups
					.entry(key)
					.or_insert_with(|| (base_color(&node.material, kind), Vec::new()))
					.1
					.push(instance);
			}
		}
	}
}

fn push_instance(
	groups: &mut BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)>,
	kit: &'static str,
	material: &MaterialRef,
	kind: VisualMaterialKind,
	placement: Placement,
	plant_origin: Vec3,
) {
	let key = PackedBatchKey::new(kit, material, kind);
	let color = base_color(material, kind);
	let entry = groups.entry(key).or_insert_with(|| (color, Vec::new()));
	entry.1.push(PackedInstance::plain(placement.affine(), plant_origin));
}

fn finish_groups(
	groups: BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)>,
) -> Vec<PackedBatch> {
	groups
		.into_iter()
		.filter(|(_, (_, instances))| !instances.is_empty())
		.map(|(key, (base_color, instances))| PackedBatch {
			key,
			base_color,
			instances: instances.into(),
		})
		.collect()
}

fn material_kind(material: &MaterialRef, _geometry: &FoliageGeometry) -> VisualMaterialKind {
	match &material.name {
		material_ref::MaterialId::Name(name) if name == STICK_MATERIAL => VisualMaterialKind::Stick,
		material_ref::MaterialId::Name(name) if name == LEAF_MATERIAL => VisualMaterialKind::Leaf,
		_ => VisualMaterialKind::Frond, // fail closed if a new call bypasses validation
	}
}

fn base_color(material: &MaterialRef, kind: VisualMaterialKind) -> Vec4 {
	if let Some(color) = material.palette.first() {
		let linear = bevy::color::LinearRgba::from(*color);
		return Vec4::new(linear.red, linear.green, linear.blue, linear.alpha);
	}
	match kind {
		VisualMaterialKind::Leaf => Vec4::new(0.22, 0.5, 0.29, 1.0),
		VisualMaterialKind::Stick => Vec4::new(0.13, 0.085, 0.055, 1.0),
		VisualMaterialKind::Frond => Vec4::new(0.22, 0.5, 0.29, 1.0),
		VisualMaterialKind::ShadowProxy => Vec4::ONE,
	}
}

fn hash_material(material: &MaterialRef) -> u64 {
	hash_bits(&MaterialRefKey::from(material))
}

fn hash_bits<T: Hash + ?Sized>(value: &T) -> u64 {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	value.hash(&mut hasher);
	hasher.finish()
}

impl PartialOrd for PackedBatchKey {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for PackedBatchKey {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.kit
			.cmp(other.kit)
			.then(self.material_kind.cmp(&other.material_kind))
			.then(self.material_hash.cmp(&other.material_hash))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{ChicoGroveHost, ForestGroveKind, ForestLayer};
	use vegetation_groves::{FlatTerrainSample, GroveExtent};
	fn tile(kind: ForestGroveKind) -> ForestGroveTile {
		if kind == ForestGroveKind::ForlornSavanna {
			use vegetation_groves::{
				forlorn_savanna::ForlornSavannaCell, grove::GroveCellVariant, ForlornSavanna,
			};
			return ForestGroveTile::ForlornSavanna(ForlornSavanna::from_placements(
				&[
					GroveCellVariant::new(
						ForlornSavannaCell::SavannaRory,
						Vec3::new(4.0, 0.0, 4.0),
						1.0,
					),
					GroveCellVariant::new(
						ForlornSavannaCell::AcaciaHighBush,
						Vec3::new(28.0, 0.0, 4.0),
						1.0,
					),
				],
				procedural_common::NoiseParams::default(),
				&GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0)),
				1,
			));
		}
		kind.grow_tile(
			GroveExtent::new(Vec3::ZERO, Vec3::new(80.0, 1.0, 80.0)),
			&FlatTerrainSample::default(),
		)
	}
	#[test]
	fn compatible_natural_woody_policies_keep_original_bands_and_trunks() -> anyhow::Result<()> {
		for kind in [
			ForestGroveKind::RiparianGeneral,
			ForestGroveKind::RollingOaks,
			ForestGroveKind::HighBush,
			ForestGroveKind::Vineyard,
			ForestGroveKind::ForlornSavanna,
		] {
			let tile = tile(kind);
			let host = ChicoGroveHost::new(tile.clone(), ForestLayer::UpperCanopy);
			let payload = super::super::cache::TilePayload::from_host(&host, 7, true)
				.map_err(|reason| anyhow::anyhow!("{} rejected: {reason:?}", tile.family_name()))?;
			assert!(payload.plant_count > 0);
			assert_eq!(Some(payload.structural_lod), tile.structural_lod());
			assert!(payload.lods[0]
				.iter()
				.any(|batch| batch.key.material_kind == VisualMaterialKind::Leaf));
			if kind == ForestGroveKind::Vineyard {
				for index in [2, 3] {
					assert!(payload.lods[index]
						.iter()
						.any(|batch| batch.key.material_kind == VisualMaterialKind::Stick));
					assert!(payload.lods[index]
						.iter()
						.filter(|batch| batch.key.material_kind == VisualMaterialKind::Stick)
						.all(|batch| batch.key.kit.contains("low")));
				}
			}
			let original = FoliageNode::merge_canopy_proxies(
				tile.proxy_foliage_nodes(LodSceneLevel::UltraLow),
			);
			let ordinary_members: usize = original
				.iter()
				.map(|node| match &node.geometry {
					FoliageGeometry::CheapBallCollection(collection) => {
						collection.placements_for_level(LodSceneLevel::UltraLow).len()
					}
					_ => 1,
				})
				.sum();
			let packed_members: usize = payload.lods[3]
				.iter()
				.filter(|batch| batch.key.material_kind == VisualMaterialKind::Leaf)
				.map(|batch| batch.instances.len())
				.sum();
			assert_eq!(packed_members, ordinary_members, "{}", tile.family_name());
		}
		Ok(())
	}
	#[test]
	fn actual_frond_tile_and_unknown_material_fail_closed() -> anyhow::Result<()> {
		let tile = tile(ForestGroveKind::DateGrove);
		assert!(tile
			.woody_lod()
			.is_some_and(|lod| lod.nest_plant_level(LodSceneLevel::Low).is_some()));
		let plants = crate::stick_physics::packed_plants(&tile)
			.ok_or_else(|| anyhow::anyhow!("missing DateGrove adapter"))?;
		assert!(plants.iter().any(|plant| {
			plant
				.foliage_nodes_for_level(LodSceneLevel::High)
				.flatten()
				.iter()
				.any(|node| node.geometry.is_frond_collection() || node.geometry.is_frond_kit())
		}));
		let host = ChicoGroveHost::new(tile, ForestLayer::UpperCanopy);
		assert!(matches!(
			super::super::cache::TilePayload::from_host(&host, 1, true),
			Err(AdmissionReason::UnsupportedGeometry)
		));
		let unknown = FoliageNode::cheap_ball(Placement::IDENTITY)
			.with_material(MaterialRef::named("unknown-leaf-lookalike"));
		assert_eq!(
			validate_nodes(&[], &[unknown], LodSceneLevel::High),
			Err(AdmissionReason::UnsupportedMaterial)
		);
		Ok(())
	}
	#[test]
	fn empty_and_tuft_are_explicit_fallbacks() {
		let orchard = vegetation_groves::Orchard {
			plants: Arc::from([]),
			structural_center: Vec3::ZERO,
			footprint_radius: 1.0,
			extent: GroveExtent::new(Vec3::ZERO, Vec3::ONE),
		};
		let host = ChicoGroveHost::new(ForestGroveTile::Orchard(orchard), ForestLayer::UpperCanopy);
		assert!(matches!(
			super::super::cache::TilePayload::from_host(&host, 1, false),
			Err(AdmissionReason::EmptyContent)
		));
		let host = ChicoGroveHost::new(tile(ForestGroveKind::CommonTufts), ForestLayer::Tufts);
		assert!(matches!(
			super::super::cache::TilePayload::from_host(&host, 1, false),
			Err(AdmissionReason::UnsupportedAdapter)
		));
	}
}
