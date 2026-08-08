//! Foliage IR node: style + geometry + placement.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Mesh3d, MeshMaterial3d, StandardMaterial, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::{lod_host_scene_pending, SceneChunk};

use crate::assets::AssetPath;
use crate::foliage::collection::{FrondCollection, FrondKit, FrondMember};
use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::probe::FoliageLodProbe;
use crate::foliage::style::FoliageStyle;
use crate::lod_band::warm_mesh_lod_culls;
use crate::lod_host::{
	posed_foliage_asset_tier, posed_frond_asset_tier, VegetationFrondAssetRoot,
};
use crate::placed::Placement;
use crate::procedural::{PendingPlaneSplay, VegetationProceduralAssets};
use crate::scene_children::{pose, posed_mesh, scene_children};

/// Authoring IR for a foliage cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct FoliageNode {
	pub style: FoliageStyle,
	pub geometry: FoliageGeometry,
	pub placement: Placement,
}

impl FoliageNode {
	pub fn new(style: FoliageStyle, geometry: FoliageGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement }
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
				FoliageLodProbe::for_frond_collection(collection)
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
		posed_mesh(
			VegetationProceduralAssets::foliage_ball(),
			VegetationProceduralAssets::foliage_material(),
			pose(self.placement),
		)
	}

	/// Stick-cylinder stand-in when a frond GLB is missing (same \(Y \in [0, 1]\) kit axis).
	///
	/// Tagged with [`VegetationFrondAssetRoot`] on the mesh entity so playground material
	/// patching can match `Added` leaves without walking the whole hierarchy every frame.
	fn procedural_frond_scene_at(&self, placement: Placement) -> impl Scene + 'static {
		let mesh = VegetationProceduralAssets::stick_cylinder();
		let material = VegetationProceduralAssets::foliage_material();
		let transform = pose(placement);
		bsn! {
			VegetationFrondAssetRoot
			Mesh3d({mesh})
			MeshMaterial3d::<StandardMaterial>({material})
			template_value(transform)
			Visibility::default()
		}
	}

	fn member_leaf_scene(&self, member: FrondMember, level: LodSceneLevel) -> Box<dyn Scene> {
		let placement = self.placement.compose_child(member.placement);
		match self.frond_glb_for_kit(member.kit, level) {
			Some(asset) => Box::new(posed_frond_asset_tier(Some(asset), pose(placement))),
			None => Box::new(self.procedural_frond_scene_at(placement)),
		}
	}

	fn collection_content(&self, collection: &FrondCollection, level: LodSceneLevel) -> Box<dyn Scene> {
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
					Some(asset) => {
						Box::new(posed_foliage_asset_tier(Some(asset), pose(self.placement)))
					}
					None => Box::new(self.procedural_ball_scene()),
				}
			}
			(
				FoliageStyle::Standard,
				FoliageGeometry::StraightFrond | FoliageGeometry::StraightFrondSegment,
			) => match self.standard_frond_glb_for_level(level) {
				Some(asset) => Box::new(posed_frond_asset_tier(Some(asset), pose(self.placement))),
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

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		match &self.geometry {
			FoliageGeometry::FrondCollection(collection) => {
				let chunks: Vec<SceneChunk> = collection
					.members_for_level(level)
					.into_iter()
					.map(|member| SceneChunk::weighted(1, self.member_leaf_scene(member, level)))
					.collect();
				if chunks.is_empty() {
					SceneChunk::primitive(scene_children(Vec::new()))
				} else {
					SceneChunk::chunks(chunks)
				}
			}
			_ => SceneChunk::primitive(self.content_for_level(level)),
		}
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		let (center, extent) = match &self.geometry {
			FoliageGeometry::FrondCollection(collection) => collection.center_and_extent(),
			_ => (
				crate::lod_band::placement_center(&self.placement),
				crate::lod_band::characteristic_extent_abs(&self.placement).max(1.0),
			),
		};
		let half = bevy::math::Vec3::splat(extent);
		let bounds = Aabb3d::from_min_max(center - half, center + half);
		lod_host_scene_pending(level, bounds)
	}
}
