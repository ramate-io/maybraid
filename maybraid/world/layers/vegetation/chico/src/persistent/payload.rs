//! One immutable High-band list built from the same ordinary posed Orchard IR.
use crate::persistent::kit::PackedKitMeshes;
use bevy::prelude::*;
use bevy::shape::Aabb3d;
use lod::LodSceneLevel;
use material_ref::MaterialRef;
use std::{
	collections::BTreeMap,
	hash::{Hash, Hasher},
	sync::Arc,
};
use vegetation_components::{
	CanopyShadowMode, FoliageGeometry, FoliageNode, Placement, StickNode, VegetationComponents,
	LEAF_MATERIAL, STICK_MATERIAL,
};
use vegetation_groves::Orchard;

pub const SHADOW_KIT: &str = "@canopy-shadow-proxy";
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaterialKind {
	Stick,
	Leaf,
	ShadowProxy,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PackedInstance {
	pub world_from_local: Mat4,
	pub plant_origin: Vec3,
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
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackedBatchKey {
	pub kit: &'static str,
	pub material_kind: MaterialKind,
	pub color: [u32; 4],
}
impl PackedBatchKey {
	pub fn new(kit: &'static str, material: &MaterialRef, kind: MaterialKind) -> Self {
		Self {
			kit,
			material_kind: kind,
			color: base_color(material, kind).to_array().map(f32::to_bits),
		}
	}
}
#[derive(Clone)]
pub struct PackedBatch {
	pub key: PackedBatchKey,
	pub base_color: Vec4,
	pub instances: Arc<[PackedInstance]>,
}
#[derive(Clone)]
pub struct SelectedBand {
	pub batches: Arc<[PackedBatch]>,
	pub source_plants: usize,
	pub source_digest: u64,
}
impl SelectedBand {
	pub fn from_orchard(
		orchard: &Orchard,
		shadows: CanopyShadowMode,
	) -> Result<Self, &'static str> {
		if orchard.plants.is_empty() {
			return Err("selected fixture contains no authored plants");
		}
		if shadows == CanopyShadowMode::Full {
			return Err("fixture requires proxy or off canopy shadow mode");
		}
		let level = LodSceneLevel::High;
		let mut groups = BTreeMap::new();
		let mut shadow_instances = Vec::new();
		for plant in orchard.placed_plants() {
			let sticks = plant.stick_nodes_for_level(level).flatten();
			let foliage = plant.foliage_nodes_for_level(level).flatten();
			validate_nodes(&sticks, &foliage, level)?;
			for node in &sticks {
				push_stick(&mut groups, node, level, plant.placement.translation);
			}
			for node in &foliage {
				push_foliage(&mut groups, node, level, plant.placement.translation);
			}
			if shadows.emit_proxies() {
				let structural = plant
					.structural_lod()
					.ok_or("fixture plant missing structural shadow source")?;
				let model = Mat4::from_scale_rotation_translation(
					Vec3::splat(structural.tree_radius.max(0.25)),
					Quat::IDENTITY,
					structural.center,
				);
				shadow_instances.push(PackedInstance::plain(model, structural.center));
			}
		}
		let mut batches = finish_groups(groups);
		if !shadow_instances.is_empty() {
			batches.push(PackedBatch {
				key: PackedBatchKey {
					kit: SHADOW_KIT,
					material_kind: MaterialKind::ShadowProxy,
					color: [1.0_f32.to_bits(); 4],
				},
				base_color: Vec4::ONE,
				instances: shadow_instances.into(),
			});
		}
		let mut digest = std::collections::hash_map::DefaultHasher::new();
		for batch in &batches {
			batch.key.hash(&mut digest);
			for instance in batch.instances.iter() {
				instance.world_from_local.to_cols_array().map(f32::to_bits).hash(&mut digest);
				instance.collection_root.to_cols_array().map(f32::to_bits).hash(&mut digest);
				instance.plant_origin.to_array().map(f32::to_bits).hash(&mut digest);
				instance.part_scale.to_bits().hash(&mut digest);
				instance.merged_leaf.hash(&mut digest);
			}
		}
		Ok(Self {
			batches: batches.into(),
			source_plants: orchard.plants.len(),
			source_digest: digest.finish(),
		})
	}
	pub fn geometric_bounds(&self, kits: &PackedKitMeshes) -> Option<Aabb3d> {
		let mut union: Option<(Vec3, Vec3)> = None;
		for batch in self.batches.iter() {
			let local = kits.ready_bounds(batch.key.kit)?;
			let local_center = Vec3::from((local.min + local.max) * 0.5);
			let half = Vec3::from((local.max - local.min) * 0.5);
			for instance in batch.instances.iter() {
				let model = instance.world_from_local;
				let center = model.transform_point3(local_center);
				let extent = model.x_axis.truncate().abs() * half.x
					+ model.y_axis.truncate().abs() * half.y
					+ model.z_axis.truncate().abs() * half.z;
				let wind = if batch.key.material_kind == MaterialKind::Leaf {
					let root = if instance.merged_leaf { instance.collection_root } else { model };
					0.108
						* root
							.x_axis
							.truncate()
							.length()
							.max(root.y_axis.truncate().length())
							.max(root.z_axis.truncate().length())
						* if instance.merged_leaf { instance.part_scale.abs() } else { 1.0 }
				} else {
					0.0
				};
				let extent = extent + Vec3::splat(wind);
				let min = center - extent;
				let max = center + extent;
				if !min.is_finite() || !max.is_finite() {
					return None;
				}
				union = Some(match union {
					Some((a, b)) => (a.min(min), b.max(max)),
					None => (min, max),
				});
			}
		}
		union.map(|(min, max)| Aabb3d::from_min_max(min, max))
	}
}
fn material_is(material: &MaterialRef, expected: &str) -> bool {
	matches!(&material.name, material_ref::MaterialId::Name(name) if name == expected)
		&& material.rasters.is_empty()
		&& material.scalars.is_empty()
}
fn validate_nodes(
	sticks: &[StickNode],
	foliage: &[FoliageNode],
	level: LodSceneLevel,
) -> Result<(), &'static str> {
	for node in sticks {
		if !material_is(&node.material, STICK_MATERIAL) {
			return Err("unsupported material");
		}
		if let Some(collection) = &node.collection {
			if collection
				.members_for_level(level)
				.iter()
				.any(|member| member.geometry.standard_glb_for_level(level).is_none())
			{
				return Err("unsupported geometry");
			}
		} else if node.geometry.standard_glb_for_level(level).is_none() {
			return Err("unsupported geometry");
		}
	}
	for node in foliage {
		if !matches!(
			node.geometry,
			FoliageGeometry::CheapBall
				| FoliageGeometry::CheapBallCollection(_)
				| FoliageGeometry::LayeredBall
		) {
			return Err("unsupported geometry");
		}
		if !material_is(&node.material, LEAF_MATERIAL) {
			return Err("unsupported material");
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
			let key = PackedBatchKey::new(asset.as_str(), &node.material, MaterialKind::Stick);
			groups
				.entry(key)
				.or_insert_with(|| (base_color(&node.material, MaterialKind::Stick), Vec::new()))
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
		MaterialKind::Stick,
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
	let kind = MaterialKind::Leaf;
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
		FoliageGeometry::StraightFrond
		| FoliageGeometry::StraightFrondSegment
		| FoliageGeometry::FrondCollection(_) => {}
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
	kind: MaterialKind,
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

fn base_color(material: &MaterialRef, kind: MaterialKind) -> Vec4 {
	if let Some(color) = material.palette.first() {
		let color = bevy::color::LinearRgba::from(*color);
		return Vec4::new(color.red, color.green, color.blue, color.alpha);
	}
	match kind {
		MaterialKind::Leaf => Vec4::new(0.22, 0.5, 0.29, 1.0),
		MaterialKind::Stick => Vec4::new(0.13, 0.085, 0.055, 1.0),
		MaterialKind::ShadowProxy => Vec4::ONE,
	}
}
