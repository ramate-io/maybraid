//! Stick IR node: style + geometry + placement.

use bevy::prelude::{Visibility, Vec3};
use bevy::scene::prelude::{bsn, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::lod_band::warm_mesh_lod_culls;
use crate::lod_host::{posed_asset_tier, warm_content_host, warm_mesh_level_host};
use crate::placed::Placement;
use crate::procedural::VegetationProceduralAssets;
use crate::scene_children::{pose, posed_mesh};
use crate::sticks::geometry::StickGeometry;
use crate::sticks::probe::StickLodProbe;
use crate::sticks::style::StickStyle;

/// Authoring IR for a stick / trunk segment.
#[derive(Debug, Clone, PartialEq)]
pub struct StickNode {
	pub style: StickStyle,
	pub geometry: StickGeometry,
	pub placement: Placement,
}

impl StickNode {
	pub fn new(style: StickStyle, geometry: StickGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement }
	}

	pub fn noisy_cylinder(geometry: StickGeometry, placement: Placement) -> Self {
		Self::new(StickStyle::NoisyCylinder, geometry, placement)
	}

	/// Branch / connector segment using `vegetation/sticks/standard/` GLBs.
	pub fn segment(placement: Placement) -> Self {
		Self::new(StickStyle::Standard, StickGeometry::Segment, placement)
	}

	/// Standard stick from a directed segment (base at `start`, along `start → end`).
	///
	/// Girth uses `radius` at the segment start. Degenerate (near-zero length) edges
	/// return [`None`].
	pub fn from_segment(start: Vec3, end: Vec3, radius: f32) -> Option<Self> {
		let ray = end - start;
		let len_sq = ray.length_squared();
		if len_sq < 1e-12 {
			return None;
		}
		let length = len_sq.sqrt();
		let placement = Placement::stick_segment(start, ray, length, radius)?;
		Some(Self::segment(placement))
	}

	/// Trunk segment using `vegetation/sticks/standard_trunk/` GLBs.
	pub fn trunk(placement: Placement) -> Self {
		Self::new(StickStyle::StandardTrunk, StickGeometry::Trunk, placement)
	}

	pub fn standard(geometry: StickGeometry, placement: Placement) -> Self {
		Self::new(StickStyle::Standard, geometry, placement)
	}

	pub fn standard_trunk(geometry: StickGeometry, placement: Placement) -> Self {
		Self::new(StickStyle::StandardTrunk, geometry, placement)
	}

	fn probe(&self) -> StickLodProbe {
		StickLodProbe::from_stick(&self.placement, self.geometry)
	}

	fn procedural_scene(&self) -> impl Scene + 'static {
		posed_mesh(
			VegetationProceduralAssets::stick_cylinder(),
			VegetationProceduralAssets::stick_material(),
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
			_ => match self.style.glb_for_level(level) {
				Some(asset) => {
					Box::new(posed_asset_tier(Some(asset), pose(self.placement))) as Box<dyn Scene>
				}
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

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		let probe = self.probe();
		match self.style {
			StickStyle::NoisyCylinder => Box::new(warm_content_host(
				level,
				probe,
				[
					(LodSceneLevel::High, Box::new(self.procedural_scene()) as Box<dyn Scene>),
					(LodSceneLevel::Medium, Box::new(self.procedural_scene()) as Box<dyn Scene>),
					(LodSceneLevel::Low, Box::new(self.procedural_scene()) as Box<dyn Scene>),
					(LodSceneLevel::UltraLow, Box::new(Self::empty_scene()) as Box<dyn Scene>),
				],
			)) as Box<dyn Scene>,
			StickStyle::Standard | StickStyle::StandardTrunk => Box::new(warm_mesh_level_host(
				level,
				probe,
				pose(self.placement),
				[
					(LodSceneLevel::High, self.style.glb_for_level(LodSceneLevel::High)),
					(LodSceneLevel::Medium, self.style.glb_for_level(LodSceneLevel::Medium)),
					(LodSceneLevel::Low, self.style.glb_for_level(LodSceneLevel::Low)),
					(LodSceneLevel::UltraLow, None),
				],
			)) as Box<dyn Scene>,
		}
	}
}
