//! Shared-kit instance lists for one orchard tile / LOD band.

use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

use bevy::math::{Mat4, Vec3, Vec4};
use lod::{LodSceneLevel, VisualLodKind, VisualLodPrimitive, VisualMaterialKind, VisualSceneChunk};
use material_ref::{MaterialRef, MaterialRefKey};
use vegetation_components::{
	FoliageGeometry, FoliageNode, Placement, StickNode, VegetationComponents, FROND_MATERIAL,
	LEAF_MATERIAL, STICK_MATERIAL,
};
use vegetation_groves::grove::{
	canopy_proxy_site, foliage_ultra_low_merged_balls, ULTRA_LOW_CANOPY_BIN_METERS,
};
use vegetation_groves::Orchard;

/// One posed kit instance. Transform is grove-local (host transform is identity).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackedInstance {
	pub world_from_local: Mat4,
	pub plant_origin: Vec3,
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
	pub instances: Vec<PackedInstance>,
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

/// Group orchard kit instances by shared GLB + material. Collections stay instanced.
pub fn pack_orchard_batches(orchard: &Orchard, level: LodSceneLevel) -> Vec<PackedBatch> {
	match level {
		LodSceneLevel::High | LodSceneLevel::Medium => pack_detail(orchard, level),
		LodSceneLevel::Low => pack_canopy(orchard, level),
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
			pack_ultra_low(orchard)
		}
	}
}

/// Planning tree the [`lod::VisualLodScene`] consume path walks.
pub fn visual_chunks_for_orchard(orchard: &Orchard, level: LodSceneLevel) -> VisualSceneChunk {
	let batches = pack_orchard_batches(orchard, level);
	if batches.is_empty() {
		return VisualSceneChunk::primitive(VisualLodPrimitive::stub(level));
	}
	VisualSceneChunk::chunks(
		batches.into_iter().map(|batch| {
			let count = batch.instances.len().max(1) as u32;
			VisualSceneChunk::weighted(count, batch.visual_primitive(level))
		}),
	)
}

fn pack_detail(orchard: &Orchard, level: LodSceneLevel) -> Vec<PackedBatch> {
	let mut groups: BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)> = BTreeMap::new();
	for plant in orchard.plants.iter() {
		let placed = plant.placed();
		let origin = plant.placement.translation;
		for node in placed.stick_nodes_for_level(level).flatten() {
			push_stick(&mut groups, &node, level, origin);
		}
		for node in placed.foliage_nodes_for_level(level).flatten() {
			push_foliage(&mut groups, &node, level, origin);
		}
	}
	finish_groups(groups)
}

fn pack_canopy(orchard: &Orchard, level: LodSceneLevel) -> Vec<PackedBatch> {
	let mut groups: BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)> = BTreeMap::new();
	for plant in orchard.plants.iter() {
		let Some(site) = canopy_proxy_site(&plant.tree, plant.placement, &plant.placed().ball_material)
		else {
			continue;
		};
		let placement = Placement::new(site.center, 0.0).with_scale(site.half_extents.max(Vec3::splat(0.25)));
		push_instance(
			&mut groups,
			FoliageGeometry::cheap_ball_glb_for_level(level).as_str(),
			&site.material,
			VisualMaterialKind::Leaf,
			placement,
			site.center,
		);
	}
	finish_groups(groups)
}

fn pack_ultra_low(orchard: &Orchard) -> Vec<PackedBatch> {
	let sites: Vec<_> = orchard
		.plants
		.iter()
		.filter_map(|plant| {
			canopy_proxy_site(&plant.tree, plant.placement, &plant.placed().ball_material)
		})
		.collect();
	let nodes = foliage_ultra_low_merged_balls(&sites, ULTRA_LOW_CANOPY_BIN_METERS);
	let mut groups: BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)> = BTreeMap::new();
	for node in nodes {
		push_foliage(&mut groups, &node, LodSceneLevel::UltraLow, node.placement.translation);
	}
	finish_groups(groups)
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
			push_instance(
				groups,
				asset.as_str(),
				&node.material,
				VisualMaterialKind::Stick,
				node.placement.compose_child(member.placement),
				plant_origin,
			);
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
	plant_origin: Vec3,
) {
	let kind = material_kind(&node.material, &node.geometry);
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
				push_instance(
					groups,
					kit,
					&node.material,
					kind,
					node.placement.compose_child(placement),
					plant_origin,
				);
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
	entry.1.push(PackedInstance { world_from_local: placement.affine(), plant_origin });
}

fn finish_groups(groups: BTreeMap<PackedBatchKey, (Vec4, Vec<PackedInstance>)>) -> Vec<PackedBatch> {
	groups
		.into_iter()
		.filter(|(_, (_, instances))| !instances.is_empty())
		.map(|(key, (base_color, instances))| PackedBatch { key, base_color, instances })
		.collect()
}

fn material_kind(material: &MaterialRef, geometry: &FoliageGeometry) -> VisualMaterialKind {
	match &material.name {
		material_ref::MaterialId::Name(name) if name == STICK_MATERIAL => VisualMaterialKind::Stick,
		material_ref::MaterialId::Name(name) if name == FROND_MATERIAL => VisualMaterialKind::Frond,
		material_ref::MaterialId::Name(name) if name == LEAF_MATERIAL => VisualMaterialKind::Leaf,
		_ if geometry.is_frond_kit() || geometry.is_frond_collection() => VisualMaterialKind::Frond,
		_ => VisualMaterialKind::Leaf,
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


/// Packed-batch identity used by tests and metrics.
pub fn packed_kind_count(chunks: &VisualSceneChunk) -> usize {
	fn walk(chunk: &VisualSceneChunk) -> usize {
		match chunk {
			VisualSceneChunk::SubChunks(children) => children.iter().map(walk).sum(),
			VisualSceneChunk::Primitive { payload, .. } => {
				usize::from(matches!(payload.kind, VisualLodKind::PackedBatch { .. }))
			}
			VisualSceneChunk::Lazy { remaining_primitives, .. } => *remaining_primitives,
		}
	}
	walk(chunks)
}
