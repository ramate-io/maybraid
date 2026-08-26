//! Foliage IR node: geometry + placement.

use std::collections::HashMap;

use bevy::light::NotShadowCaster;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Vec3};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{
	cull_offset_bands_from_factor, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus,
};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use material_ref::{MaterialId, MaterialRef};

use crate::assets::AssetPath;
use crate::foliage::ball_collection::{
	CheapBallCollection, CHEAP_BALL_COLLECTION_HIGH_METERS, CHEAP_BALL_COLLECTION_LOW_METERS,
	CHEAP_BALL_COLLECTION_MEDIUM_METERS,
};
use crate::foliage::collection::{
	FrondCollection, FrondKit, FROND_COLLECTION_HIGH_METERS, FROND_COLLECTION_LOW_METERS,
	FROND_COLLECTION_MEDIUM_METERS,
};
use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::probe::FoliageLodProbe;
use crate::lod_host::{
	posed_foliage_asset_tier, posed_foliage_multi_scene_merge, posed_frond_asset_tier,
	posed_frond_multi_scene_merge,
};
use crate::materials::chico_frond_material_ref;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children};
use scene_ref::{MultiSceneMerge, MultiScenePart};

/// Authoring IR for a foliage cluster — also the fine-phase [`LodScene`] host component.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct FoliageNode {
	pub geometry: FoliageGeometry,
	pub placement: Placement,
	/// Deferred material. Defaults to [`MaterialRef::default()`] (green standard);
	/// frond constructors stamp [`crate::chico_frond_material_ref`]; higher-order
	/// types set leaf / palette as needed.
	pub material: MaterialRef,
}

impl FoliageNode {
	pub fn new(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self { geometry, placement, material: MaterialRef::default() }
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = material;
		self
	}

	/// Layered ball using `vegetation/foliage/standard/layered_ball_001_*` GLBs.
	pub fn layered_ball(placement: Placement) -> Self {
		Self::new(FoliageGeometry::LayeredBall, placement)
	}

	/// Cheap ball using `vegetation/foliage/standard/cheap_ball_001_*` GLBs.
	///
	/// Prefer for dense packed clusters where silhouette comes from density.
	pub fn cheap_ball(placement: Placement) -> Self {
		Self::new(FoliageGeometry::CheapBall, placement)
	}

	/// Square-ended straight frond segment (`straight_frond_segment_001_*`).
	pub fn straight_frond_segment(placement: Placement) -> Self {
		Self::new(FoliageGeometry::StraightFrondSegment, placement)
			.with_material(chico_frond_material_ref())
	}

	/// Point-tip straight frond (`straight_frond_001_*`); prefer [`Self::straight_frond_segment`].
	pub fn straight_frond(placement: Placement) -> Self {
		Self::new(FoliageGeometry::StraightFrond, placement)
			.with_material(chico_frond_material_ref())
	}

	/// Frond collection under one LOD parent (merge thinning by collection extent).
	///
	/// Parent [`Placement`] is usually identity when members are already tree-local.
	pub fn frond_collection(collection: FrondCollection, placement: Placement) -> Self {
		Self::new(FoliageGeometry::FrondCollection(collection), placement)
			.with_material(chico_frond_material_ref())
	}

	/// Cheap-ball collection under one LOD parent (merge thinning by collection extent).
	pub fn cheap_ball_collection(collection: CheapBallCollection, placement: Placement) -> Self {
		Self::new(FoliageGeometry::CheapBallCollection(collection), placement)
	}

	/// Fold cheap-ball nodes into one collection (shared material, baked probe).
	pub fn merge_cheap_balls(nodes: impl IntoIterator<Item = Self>) -> Option<Self> {
		let nodes: Vec<Self> = nodes.into_iter().collect();
		let material = nodes.first()?.material.clone();
		let mut placements = Vec::with_capacity(nodes.len());
		for node in &nodes {
			match &node.geometry {
				FoliageGeometry::CheapBallCollection(existing) => {
					placements.extend(existing.placements.iter().copied());
				}
				FoliageGeometry::CheapBall => placements.push(node.placement),
				_ => {}
			}
		}
		if placements.is_empty() {
			return None;
		}
		let collection = CheapBallCollection::new(placements).bake_bounds_from_placements();
		Some(Self::cheap_ball_collection(collection, Placement::IDENTITY).with_material(material))
	}

	/// Fold cheap balls into one collection **per material recipe**; leave fronds
	/// and other geometries as-is.
	///
	/// Grove Low / UltraLow canopy proxies use this so a tile is a few posed kits,
	/// not one [`lod::LodScene`] host per plant. Trunk (stick) and crown (leaf)
	/// must not share a kit — [`Self::merge_cheap_balls`] keeps the first
	/// material, which painted umbrellas bark-colored.
	pub fn merge_canopy_proxies(nodes: impl IntoIterator<Item = Self>) -> Vec<Self> {
		let mut groups: HashMap<MaterialId, Vec<Self>> = HashMap::new();
		let mut rest = Vec::new();
		for node in nodes {
			match &node.geometry {
				FoliageGeometry::CheapBall | FoliageGeometry::CheapBallCollection(_) => {
					groups.entry(node.material.name.clone()).or_default().push(node);
				}
				_ => rest.push(node),
			}
		}
		for cheap in groups.into_values() {
			if let Some(merged) = Self::merge_cheap_balls(cheap) {
				rest.insert(0, merged);
			}
		}
		rest
	}

	/// Expand the cheap-ball collection probe to at least `radius` around `center`.
	pub fn with_cheap_ball_probe(mut self, center: Vec3, radius: f32) -> Self {
		if let FoliageGeometry::CheapBallCollection(collection) = &mut self.geometry {
			collection.center = center;
			collection.radius = radius.max(collection.radius).max(1e-4);
		}
		self
	}

	pub fn standard(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self::new(geometry, placement)
	}

	fn probe(&self) -> FoliageLodProbe {
		match &self.geometry {
			FoliageGeometry::FrondCollection(collection) => {
				let (local_center, local_radius) = collection.center_and_extent();
				let (center, extent) = self.composed_collection_extent(local_center, local_radius);
				let mut probe = FoliageLodProbe::for_kit_collection(center, extent);
				probe.center = center;
				probe.extent = extent;
				probe
			}
			FoliageGeometry::CheapBallCollection(collection) => {
				let (local_center, local_radius) = collection.center_and_extent();
				let (center, extent) = self.composed_collection_extent(local_center, local_radius);
				FoliageLodProbe::for_cheap_ball_probe(center, extent)
			}
			_ => FoliageLodProbe::from_placement(&self.placement),
		}
	}

	fn composed_collection_extent(&self, local_center: Vec3, local_radius: f32) -> (Vec3, f32) {
		let world_center =
			self.placement.compose_child(Placement::new(local_center, 0.0)).translation;
		let scale = self.placement.scale.abs().max_element().max(1e-4);
		(world_center, (local_radius * scale).max(1e-4))
	}

	fn standard_ball_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match &self.geometry {
			FoliageGeometry::LayeredBall => {
				Some(FoliageGeometry::layered_ball_glb_for_level(level))
			}
			FoliageGeometry::CheapBall => Some(FoliageGeometry::cheap_ball_glb_for_level(level)),
			_ => None,
		}
	}

	fn standard_frond_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match &self.geometry {
			FoliageGeometry::StraightFrond => {
				Some(FoliageGeometry::straight_frond_glb_for_level(level))
			}
			FoliageGeometry::StraightFrondSegment => {
				Some(FoliageGeometry::straight_frond_segment_glb_for_level(level))
			}
			_ => None,
		}
	}

	fn frond_glb_for_kit(kit: FrondKit, level: LodSceneLevel) -> AssetPath {
		match kit {
			FrondKit::StraightFrond => FoliageGeometry::straight_frond_glb_for_level(level),
			FrondKit::StraightFrondSegment => {
				FoliageGeometry::straight_frond_segment_glb_for_level(level)
			}
		}
	}

	/// Collection-local [`MultiSceneMerge`] (member poses only — parent transform separate).
	fn collection_multi_scene_merge(
		&self,
		collection: &FrondCollection,
		level: LodSceneLevel,
	) -> Option<MultiSceneMerge> {
		let members = collection.members_for_level(level);
		if members.is_empty() {
			return None;
		}
		let mut parts = Vec::with_capacity(members.len());
		for member in members {
			let asset = Self::frond_glb_for_kit(member.kit, level);
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(member.placement)));
		}
		Some(MultiSceneMerge::new(parts))
	}

	fn cheap_ball_multi_scene_merge(
		&self,
		collection: &CheapBallCollection,
		level: LodSceneLevel,
	) -> Option<MultiSceneMerge> {
		let placements = collection.placements_for_level(level);
		if placements.is_empty() {
			return None;
		}
		let asset = FoliageGeometry::cheap_ball_glb_for_level(level);
		let mut parts = Vec::with_capacity(placements.len());
		for placement in placements {
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(placement)));
		}
		Some(MultiSceneMerge::new(parts))
	}

	fn collection_content(
		&self,
		collection: &FrondCollection,
		level: LodSceneLevel,
	) -> Box<dyn Scene> {
		if let Some(merge) = self.collection_multi_scene_merge(collection, level) {
			return Box::new(posed_frond_multi_scene_merge(
				merge,
				pose(self.placement),
				self.material.clone(),
			));
		}
		Box::new(scene_children(Vec::new()))
	}

	fn cheap_ball_collection_content(
		&self,
		collection: &CheapBallCollection,
		level: LodSceneLevel,
	) -> Box<dyn Scene> {
		if let Some(merge) = self.cheap_ball_multi_scene_merge(collection, level) {
			return Box::new(posed_foliage_multi_scene_merge(
				merge,
				pose(self.placement),
				self.material.clone(),
			));
		}
		Box::new((bsn! { NotShadowCaster }, scene_children(Vec::new())))
	}

	fn content_for_level(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		match &self.geometry {
			FoliageGeometry::CheapBall => Box::new((
				bsn! { NotShadowCaster },
				posed_foliage_asset_tier(
					self.standard_ball_glb_for_level(level),
					pose(self.placement),
					self.material.clone(),
				),
			)),
			FoliageGeometry::LayeredBall => Box::new(posed_foliage_asset_tier(
				self.standard_ball_glb_for_level(level),
				pose(self.placement),
				self.material.clone(),
			)),
			FoliageGeometry::StraightFrond | FoliageGeometry::StraightFrondSegment => {
				Box::new(posed_frond_asset_tier(
					self.standard_frond_glb_for_level(level),
					pose(self.placement),
					self.material.clone(),
				))
			}
			FoliageGeometry::FrondCollection(collection) => {
				self.collection_content(collection, level)
			}
			FoliageGeometry::CheapBallCollection(collection) => {
				self.cheap_ball_collection_content(collection, level)
			}
		}
	}
}

impl LodScene for FoliageNode {
	fn host_contents(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
		let host = self.clone();
		let probe = self.probe();
		bsn! {
			template_value(host)
			template_value(probe)
		}
	}

	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		// Frond collections: absolute-meter warm-root cull (independent of radius-relative spawn).
		if self.geometry.is_kit_collection() {
			let probe = self.probe();
			let distance = lod_ref.current_transform.translation.distance(probe.center);
			let (high_m, mid_m, low_m) = match &self.geometry {
				FoliageGeometry::CheapBallCollection(_) => (
					CHEAP_BALL_COLLECTION_HIGH_METERS,
					CHEAP_BALL_COLLECTION_MEDIUM_METERS,
					CHEAP_BALL_COLLECTION_LOW_METERS,
				),
				_ => (
					FROND_COLLECTION_HIGH_METERS,
					FROND_COLLECTION_MEDIUM_METERS,
					FROND_COLLECTION_LOW_METERS,
				),
			};
			// Keep all warm roots while still inside the High cull band (~500 m).
			// `cull_offset_bands_from_factor` alone still lists Low/UltraLow in High, which
			// would defeat the `LodSceneCulls::None` short-circuit in cull enqueue.
			if distance <= high_m {
				return LodSceneCulls::None;
			}
			return cull_offset_bands_from_factor(distance, high_m, mid_m, low_m);
		}

		let probe = self.probe();
		let factor = probe.band_metric(lod_ref.current_transform.translation);
		if factor <= probe.high_factor {
			return LodSceneCulls::None;
		}
		cull_offset_bands_from_factor(
			factor,
			probe.high_factor,
			probe.medium_factor,
			probe.low_factor,
		)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		match &self.geometry {
			FoliageGeometry::FrondCollection(collection) => {
				// One chunk: merged mesh (or empty). Avoid per-leaf chunks so fulfill
				// schedules the archetypal MultiSceneMerge as a unit.
				SceneChunk::primitive(self.collection_content(collection, level))
			}
			FoliageGeometry::CheapBallCollection(collection) => {
				SceneChunk::primitive(self.cheap_ball_collection_content(collection, level))
			}
			_ => SceneChunk::primitive(self.content_for_level(level)),
		}
	}

	fn scene_bounds(&self) -> Aabb3d {
		let (center, extent) = match &self.geometry {
			FoliageGeometry::FrondCollection(_) | FoliageGeometry::CheapBallCollection(_) => {
				let probe = self.probe();
				(probe.center, probe.extent.max(1.0))
			}
			_ => (
				crate::lod_band::placement_center(&self.placement),
				crate::lod_band::characteristic_extent_abs(&self.placement).max(1.0),
			),
		};
		let half = bevy::math::Vec3::splat(extent);
		Aabb3d::from_min_max(center - half, center + half)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::materials::{chico_frond_material_ref, CHICO_FROND_MATERIAL};
	use material_ref::MaterialId;

	#[test]
	fn frond_constructors_use_chico_frond_material() {
		let expected = MaterialId::named(CHICO_FROND_MATERIAL);
		assert_eq!(
			FoliageNode::straight_frond_segment(Placement::IDENTITY).material.name,
			expected
		);
		assert_eq!(FoliageNode::straight_frond(Placement::IDENTITY).material.name, expected);
		assert_eq!(
			FoliageNode::frond_collection(FrondCollection::new([]), Placement::IDENTITY)
				.material
				.name,
			expected
		);
		assert_eq!(chico_frond_material_ref().name, expected);
	}
}
