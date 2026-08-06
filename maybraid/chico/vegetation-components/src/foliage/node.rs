//! Foliage IR node: style + geometry + placement.

use bevy::prelude::Visibility;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::assets::AssetPath;
use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::probe::FoliageLodProbe;
use crate::foliage::style::FoliageStyle;
use crate::lod_band::warm_mesh_lod_culls;
use crate::lod_host::{
	posed_foliage_asset_tier, posed_frond_asset_tier, warm_content_host_hsl,
	warm_foliage_mesh_level_host, warm_frond_mesh_level_host,
};
use crate::placed::Placement;
use crate::procedural::{PendingPlaneSplay, VegetationProceduralAssets};
use crate::scene_children::{pose, posed_mesh};

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

	pub fn standard(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self::new(FoliageStyle::Standard, geometry, placement)
	}

	pub fn plane_splay(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self::new(FoliageStyle::PlaneSplay, geometry, placement)
	}

	fn standard_ball_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match self.geometry {
			FoliageGeometry::LayeredBall => self.style.layered_ball_glb_for_level(level),
			FoliageGeometry::CheapBall => self.style.cheap_ball_glb_for_level(level),
			_ => None,
		}
	}

	pub(crate) fn standard_frond_glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match self.geometry {
			FoliageGeometry::StraightFrond => self.style.straight_frond_glb_for_level(level),
			FoliageGeometry::StraightFrondSegment => {
				self.style.straight_frond_segment_glb_for_level(level)
			}
			_ => None,
		}
	}

	/// Posed frond mesh for a collection host (no nested per-frond LOD host).
	pub(crate) fn collection_leaf_scene(&self, level: LodSceneLevel) -> Box<dyn Scene> {
		match self.standard_frond_glb_for_level(level) {
			Some(asset) => Box::new(posed_frond_asset_tier(Some(asset), pose(self.placement))),
			None => Box::new(self.procedural_frond_scene()),
		}
	}

	fn is_standard_ball(&self) -> bool {
		matches!(
			(&self.style, &self.geometry),
			(
				FoliageStyle::Standard,
				FoliageGeometry::LayeredBall | FoliageGeometry::CheapBall
			)
		)
	}

	fn is_standard_frond(&self) -> bool {
		matches!(
			(&self.style, &self.geometry),
			(
				FoliageStyle::Standard,
				FoliageGeometry::StraightFrond | FoliageGeometry::StraightFrondSegment
			)
		)
	}

	fn procedural_ball_scene(&self) -> impl Scene + 'static {
		posed_mesh(
			VegetationProceduralAssets::foliage_ball(),
			VegetationProceduralAssets::foliage_material(),
			pose(self.placement),
		)
	}

	/// Stick-cylinder stand-in when a frond GLB is missing (same \(Y \in [0, 1]\) kit axis).
	fn procedural_frond_scene(&self) -> impl Scene + 'static {
		posed_mesh(
			VegetationProceduralAssets::stick_cylinder(),
			VegetationProceduralAssets::foliage_material(),
			pose(self.placement),
		)
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
				None => Box::new(self.procedural_frond_scene()),
			},
			_ => Box::new(self.procedural_ball_scene()),
		}
	}
}

impl LodScene for FoliageNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		FoliageLodProbe::from_placement(&self.placement).level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		FoliageLodProbe::from_placement(&self.placement).status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		let probe = FoliageLodProbe::from_placement(&self.placement);
		if self.is_standard_ball() {
			Box::new(warm_foliage_mesh_level_host(
				level,
				probe,
				pose(self.placement),
				[
					(
						LodSceneLevel::High,
						self.standard_ball_glb_for_level(LodSceneLevel::High),
					),
					(
						LodSceneLevel::Medium,
						self.standard_ball_glb_for_level(LodSceneLevel::Medium),
					),
					(
						LodSceneLevel::Low,
						self.standard_ball_glb_for_level(LodSceneLevel::Low),
					),
				],
			)) as Box<dyn Scene>
		} else if self.is_standard_frond() {
			Box::new(warm_frond_mesh_level_host(
				level,
				probe,
				pose(self.placement),
				[
					(
						LodSceneLevel::High,
						self.standard_frond_glb_for_level(LodSceneLevel::High),
					),
					(
						LodSceneLevel::Medium,
						self.standard_frond_glb_for_level(LodSceneLevel::Medium),
					),
					(
						LodSceneLevel::Low,
						self.standard_frond_glb_for_level(LodSceneLevel::Low),
					),
				],
			)) as Box<dyn Scene>
		} else {
			Box::new(warm_content_host_hsl(
				level,
				probe,
				self.content_for_level(LodSceneLevel::High),
				self.content_for_level(LodSceneLevel::Medium),
				self.content_for_level(LodSceneLevel::Low),
			)) as Box<dyn Scene>
		}
	}
}
