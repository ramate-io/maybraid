//! Foliage IR node: style + geometry + placement.

use bevy::prelude::Visibility;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;

use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::probe::FoliageLodProbe;
use crate::foliage::style::FoliageStyle;
use crate::lod_band::warm_mesh_lod_culls;
use crate::lod_host::warm_content_host_hsl;
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

	pub fn plane_splay(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self::new(FoliageStyle::PlaneSplay, geometry, placement)
	}

	fn content_scene(&self) -> Box<dyn Scene> {
		match (&self.style, &self.geometry) {
			(FoliageStyle::PlaneSplay, FoliageGeometry::PlaneSplay {
				icosphere_subdivisions,
				core_radius,
				leaf_disc_radius,
			}) => {
				let pending = PendingPlaneSplay {
					icosphere_subdivisions: *icosphere_subdivisions,
					core_radius: *core_radius,
					leaf_disc_radius: *leaf_disc_radius,
				};
				let transform = pose(self.placement);
				Box::new(bsn! {
					template_value(pending)
					template_value(transform)
					Visibility::default()
				})
			}
			_ => Box::new(posed_mesh(
				VegetationProceduralAssets::foliage_ball(),
				VegetationProceduralAssets::foliage_material(),
				pose(self.placement),
			)),
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

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		self.content_scene()
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		let probe = FoliageLodProbe::from_placement(&self.placement);
		warm_content_host_hsl(
			level,
			probe,
			self.content_scene(),
			self.content_scene(),
			self.content_scene(),
		)
	}
}
