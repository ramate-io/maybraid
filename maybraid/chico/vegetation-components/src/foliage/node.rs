//! Foliage IR node: style + geometry + placement.

use bevy::light::NotShadowCaster;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Mesh3d, MeshMaterial3d, StandardMaterial, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{
	cull_offset_bands_from_factor, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus,
};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use material_ref::{MaterialRef, MaterialRefRoot};

use crate::assets::AssetPath;
use crate::foliage::collection::{
	FrondCollection, FrondKit, FrondMember, FROND_COLLECTION_HIGH_METERS,
	FROND_COLLECTION_LOW_METERS, FROND_COLLECTION_MEDIUM_METERS,
};
use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::probe::FoliageLodProbe;
use crate::foliage::style::FoliageStyle;
use crate::lod_host::{
	posed_foliage_asset_tier, posed_frond_asset_tier, posed_frond_multi_scene_merge,
};
use crate::materials::chico_leaf_material_ref;
use crate::placed::Placement;
use crate::procedural::{PendingPlaneSplay, VegetationProceduralAssets};
use crate::scene_children::{pose, posed_mesh_material_ref, scene_children};
use scene_ref::{MultiSceneMerge, MultiScenePart};

/// Authoring IR for a foliage cluster — also the fine-phase [`LodScene`] host component.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct FoliageNode {
	pub style: FoliageStyle,
	pub geometry: FoliageGeometry,
	pub placement: Placement,
	/// Deferred material (`MaterialRef::default()` = green standard; canopy constructors use leaf).
	pub material: MaterialRef,
}

impl FoliageNode {
	pub fn new(style: FoliageStyle, geometry: FoliageGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
			material: chico_leaf_material_ref(),
		}
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = material;
		self
	}

	pub fn noisy_ball(placement: Placement) -> Self {
		Self::new(FoliageStyle::NoisyBall, FoliageGeometry::UnitBall, placement)
	}

	/// Layered ball using `vegetation/foliage/standard/layered_ball_001_*` GLBs.
	pub fn layered_ball(placement: Placement) -> Self {
		Self::new(FoliageStyle::Standard, FoliageGeometry::LayeredBall, placement)
	}

	/// Cheap ball using `vegetation/foliage/standard/cheap_ball_001_*` GLBs.
	///
	/// Prefer for dense packed clusters where silhouette comes from density.
	pub fn cheap_ball(placement: Placement) -> Self {
		Self::new(FoliageStyle::Standard, FoliageGeometry::CheapBall, placement)
	}

	/// Square-ended straight frond segment (`straight_frond_segment_001_*`).
	pub fn straight_frond_segment(placement: Placement) -> Self {
		Self::new(
			FoliageStyle::Standard,
			FoliageGeometry::StraightFrondSegment,
			placement,
		)
	}

	/// Point-tip straight frond (`straight_frond_001_*`); prefer [`Self::straight_frond_segment`].
	pub fn straight_frond(placement: Placement) -> Self {
		Self::new(FoliageStyle::Standard, FoliageGeometry::StraightFrond, placement)
	}

	/// Frond collection under one LOD parent (merge thinning by collection extent).
	///
	/// Parent [`Placement`] is usually identity when members are already tree-local.
	pub fn frond_collection(collection: FrondCollection, placement: Placement) -> Self {
		Self::new(
			FoliageStyle::Standard,
			FoliageGeometry::FrondCollection(collection),
			placement,
		)
	}

	pub fn standard(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self::new(FoliageStyle::Standard, geometry, placement)
	}

	pub fn plane_splay(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self::new(FoliageStyle::PlaneSplay, geometry, placement)
	}

	fn probe(&self) -> FoliageLodProbe {
		match &self.geometry {
			FoliageGeometry::FrondCollection(collection) => {
				// Collection center/radius stay unit-/collection-local; compose plant pose.
				let (local_center, local_radius) = collection.center_and_extent();
				let world_center = self
					.placement
					.compose_child(Placement::new(local_center, 0.0))
					.translation;
				let scale = self.placement.scale.abs().max_element().max(1e-4);
				let mut probe = FoliageLodProbe::for_frond_collection(collection);
				probe.center = world_center;
				probe.extent = local_radius * scale;
				probe
			}
			_ => FoliageLodProbe::from_placement(&self.placement),
		}
	}

	fn standard_ball_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match &self.geometry {
			FoliageGeometry::LayeredBall => self.style.layered_ball_glb_for_level(level),
			FoliageGeometry::CheapBall => self.style.cheap_ball_glb_for_level(level),
			_ => None,
		}
	}

	fn standard_frond_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match &self.geometry {
			FoliageGeometry::StraightFrond => self.style.straight_frond_glb_for_level(level),
			FoliageGeometry::StraightFrondSegment => {
				self.style.straight_frond_segment_glb_for_level(level)
			}
			_ => None,
		}
	}

	fn frond_glb_for_kit(&self, kit: FrondKit, level: LodSceneLevel) -> Option<AssetPath> {
		match kit {
			FrondKit::StraightFrond => self.style.straight_frond_glb_for_level(level),
			FrondKit::StraightFrondSegment => {
				self.style.straight_frond_segment_glb_for_level(level)
			}
		}
	}

	fn procedural_ball_scene(&self) -> impl Scene + 'static {
		posed_mesh_material_ref(
			VegetationProceduralAssets::foliage_ball(),
			VegetationProceduralAssets::foliage_material(),
			self.material.clone(),
			pose(self.placement),
		)
	}

	/// Stick-cylinder stand-in when a frond GLB is missing (same \(Y \in [0, 1]\) kit axis).
	fn procedural_frond_scene_at(&self, placement: Placement) -> impl Scene + 'static {
		let mesh = VegetationProceduralAssets::stick_cylinder();
		let placeholder = VegetationProceduralAssets::foliage_material();
		let material = self.material.clone();
		let transform = pose(placement);
		bsn! {
			NotShadowCaster
			Mesh3d({mesh})
			MeshMaterial3d::<StandardMaterial>({placeholder})
			template_value(MaterialRefRoot(material))
			template_value(transform)
			Visibility::default()
		}
	}

	fn member_leaf_scene(&self, member: FrondMember, level: LodSceneLevel) -> Box<dyn Scene> {
		let placement = self.placement.compose_child(member.placement);
		match self.frond_glb_for_kit(member.kit, level) {
			Some(asset) => Box::new(posed_frond_asset_tier(
				Some(asset),
				pose(placement),
				self.material.clone(),
			)),
			None => Box::new(self.procedural_frond_scene_at(placement)),
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
			let asset = self.frond_glb_for_kit(member.kit, level)?;
			parts.push(MultiScenePart::new(asset.scene_ref(), pose(member.placement)));
		}
		Some(MultiSceneMerge::new(parts))
	}

	fn collection_content(&self, collection: &FrondCollection, level: LodSceneLevel) -> Box<dyn Scene> {
		if let Some(merge) = self.collection_multi_scene_merge(collection, level) {
			return Box::new(posed_frond_multi_scene_merge(
				merge,
				pose(self.placement),
				self.material.clone(),
			));
		}
		// Procedural / missing-GLB fallback: per-member children (world-composed).
		let children: Vec<Box<dyn Scene>> = collection
			.members_for_level(level)
			.into_iter()
			.map(|member| self.member_leaf_scene(member, level))
			.collect();
		Box::new(scene_children(children))
	}

	fn plane_splay_scene(
		&self,
		icosphere_subdivisions: u32,
		core_radius: f32,
		leaf_disc_radius: f32,
	) -> impl Scene + 'static {
		let pending = PendingPlaneSplay {
			icosphere_subdivisions,
			core_radius,
			leaf_disc_radius,
		};
		let transform = pose(self.placement);
		bsn! {
			template_value(pending)
			template_value(transform)
			Visibility::default()
		}
	}

	fn content_for_level(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		match (&self.style, &self.geometry) {
			(FoliageStyle::PlaneSplay, FoliageGeometry::PlaneSplay {
				icosphere_subdivisions,
				core_radius,
				leaf_disc_radius,
			}) => Box::new(self.plane_splay_scene(
				*icosphere_subdivisions,
				*core_radius,
				*leaf_disc_radius,
			)),
			(FoliageStyle::Standard, FoliageGeometry::LayeredBall | FoliageGeometry::CheapBall) => {
				match self.standard_ball_glb_for_level(level) {
					Some(asset) => Box::new(posed_foliage_asset_tier(
						Some(asset),
						pose(self.placement),
						self.material.clone(),
					)),
					None => Box::new(self.procedural_ball_scene()),
				}
			}
			(
				FoliageStyle::Standard,
				FoliageGeometry::StraightFrond | FoliageGeometry::StraightFrondSegment,
			) => match self.standard_frond_glb_for_level(level) {
				Some(asset) => Box::new(posed_frond_asset_tier(
					Some(asset),
					pose(self.placement),
					self.material.clone(),
				)),
				None => Box::new(self.procedural_frond_scene_at(self.placement)),
			},
			(FoliageStyle::Standard, FoliageGeometry::FrondCollection(collection)) => {
				self.collection_content(collection, level)
			}
			_ => Box::new(self.procedural_ball_scene()),
		}
	}
}

impl LodScene for FoliageNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		// Frond collections: absolute-meter warm-root cull (independent of radius-relative spawn).
		if let FoliageGeometry::FrondCollection(_) = &self.geometry {
			let probe = self.probe();
			let distance = lod_ref.current_transform.translation.distance(probe.center);
			// Keep all warm roots while still inside the High cull band (~500 m).
			// `cull_offset_bands_from_factor` alone still lists Low/UltraLow in High, which
			// would defeat the `LodSceneCulls::None` short-circuit in cull enqueue.
			if distance <= FROND_COLLECTION_HIGH_METERS {
				return LodSceneCulls::None;
			}
			return cull_offset_bands_from_factor(
				distance,
				FROND_COLLECTION_HIGH_METERS,
				FROND_COLLECTION_MEDIUM_METERS,
				FROND_COLLECTION_LOW_METERS,
			);
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
			_ => SceneChunk::primitive(self.content_for_level(level)),
		}
	}

	fn scene_bounds(&self) -> Aabb3d {
		let (center, extent) = match &self.geometry {
			FoliageGeometry::FrondCollection(_) => {
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
