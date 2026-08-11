//! Stick IR node: style + geometry + placement.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Visibility, Vec3};
use bevy::scene::prelude::{bsn, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::SceneChunk;
use material_ref::MaterialRef;

use crate::assets::AssetPath;
use crate::lod_band::warm_mesh_lod_culls;
use crate::lod_host::posed_material_asset_tier;
use crate::placed::Placement;
use crate::procedural::VegetationProceduralAssets;
use crate::scene_children::{pose, posed_mesh_material_ref};
use crate::sticks::geometry::StickGeometry;
use crate::sticks::probe::StickLodProbe;
use crate::sticks::style::StickStyle;

/// Authoring IR for a stick / trunk segment — also the fine-phase [`LodScene`] host component.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct StickNode {
	pub style: StickStyle,
	pub geometry: StickGeometry,
	pub placement: Placement,
	/// Deferred material. Defaults to [`MaterialRef::default()`] (green standard);
	/// higher-order types set stick / palette as needed.
	pub material: MaterialRef,
}

impl StickNode {
	pub fn new(style: StickStyle, geometry: StickGeometry, placement: Placement) -> Self {
		Self {
			style,
			geometry,
			placement,
			material: MaterialRef::default(),
		}
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = material;
		self
	}

	pub fn noisy_cylinder(geometry: StickGeometry, placement: Placement) -> Self {
		Self::new(StickStyle::NoisyCylinder, geometry, placement)
	}

	/// Branch / connector segment using `vegetation/sticks/standard/001_*` GLBs.
	pub fn segment(placement: Placement) -> Self {
		Self::new(StickStyle::Standard, StickGeometry::Segment, placement)
	}

	/// Standard stick from a directed segment (base at `start`, along `start → end`).
	///
	/// Girth uses `radius` at the segment start. Degenerate (near-zero length) edges
	/// return [`None`]. Defaults to [`StickGeometry::Segment`].
	pub fn from_segment(start: Vec3, end: Vec3, radius: f32) -> Option<Self> {
		Self::from_segment_geometry(start, end, radius, StickGeometry::Segment)
	}

	/// Like [`Self::from_segment`], with an explicit geometry (segment vs trunk kit).
	pub fn from_segment_geometry(
		start: Vec3,
		end: Vec3,
		radius: f32,
		geometry: StickGeometry,
	) -> Option<Self> {
		let ray = end - start;
		let len_sq = ray.length_squared();
		if len_sq < 1e-12 {
			return None;
		}
		let length = len_sq.sqrt();
		let placement = Placement::stick_segment(start, ray, length, radius)?;
		Some(Self::standard(geometry, placement))
	}

	/// Trunk geometry using `vegetation/sticks/standard/trunk_001_*` GLBs.
	pub fn trunk(placement: Placement) -> Self {
		Self::new(StickStyle::Standard, StickGeometry::Trunk, placement)
	}

	pub fn standard(geometry: StickGeometry, placement: Placement) -> Self {
		Self::new(StickStyle::Standard, geometry, placement)
	}

	fn probe(&self) -> StickLodProbe {
		StickLodProbe::from_stick(&self.placement, self.geometry)
	}

	fn glb_for_level(&self, level: LodSceneLevel) -> Option<AssetPath> {
		match self.style {
			StickStyle::NoisyCylinder => None,
			StickStyle::Standard => self.geometry.standard_glb_for_level(level),
		}
	}

	fn procedural_scene(&self) -> impl Scene + 'static {
		posed_mesh_material_ref(
			VegetationProceduralAssets::stick_cylinder(),
			VegetationProceduralAssets::stick_material(),
			self.material.clone(),
			pose(self.placement),
		)
	}

	fn empty_scene() -> impl Scene + 'static {
		bsn! {
			Visibility::Inherited
		}
	}

	fn content_for_level(&self, level: LodSceneLevel) -> impl Scene + 'static {
		match level {
			LodSceneLevel::UltraLow => Box::new(Self::empty_scene()) as Box<dyn Scene>,
			_ => match self.glb_for_level(level) {
				Some(asset) => Box::new(posed_material_asset_tier(
					Some(asset),
					pose(self.placement),
					Some(self.material.clone()),
				)) as Box<dyn Scene>,
				None => Box::new(self.procedural_scene()) as Box<dyn Scene>,
			},
		}
	}
}

impl LodScene for StickNode {
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
		SceneChunk::primitive(self.content_for_level(level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		let center = crate::lod_band::placement_center(&self.placement);
		let extent = crate::lod_band::characteristic_extent_abs(&self.placement).max(1.0);
		let half = Vec3::splat(extent);
		Aabb3d::from_min_max(center - half, center + half)
	}
}
