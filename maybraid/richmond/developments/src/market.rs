//! Connected Old City Market sites, terraces, stalls, and lanes.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, JointNode, Layers};
use richmond_buildings::{Openings, RectFloor, RectFloorParams, RectFloorSlab};

use crate::{BuildingFootprint, ConnectedDevelopment, PlacedBuilding, ShepherdsVillageBuilding};

pub const MARKET_PLATFORM_HEIGHT: f32 = 0.35;

/// Stall-count tier assigned from graph topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OldCityMarketTier {
	Dense,
	Medium,
	Sparse,
}

/// One shared stone platform underneath a cluster of market buildings.
#[derive(Debug, Clone, PartialEq)]
pub struct OldCityMarketTerrace {
	pub bounds: Aabb3d,
	shell: RectFloor,
}

impl OldCityMarketTerrace {
	pub fn new(center: Vec2, footprint: Vec2, elevation: f32) -> Self {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - footprint.x * 0.5, elevation, center.y - footprint.y * 0.5),
			Vec3::new(
				center.x + footprint.x * 0.5,
				elevation + MARKET_PLATFORM_HEIGHT,
				center.y + footprint.y * 0.5,
			),
		);
		let shell = RectFloorParams::new(
			Vec3::new(center.x, elevation, center.y),
			footprint,
			MARKET_PLATFORM_HEIGHT,
		)
		.floor(RectFloorSlab::Solid)
		.ceiling(RectFloorSlab::Solid)
		.style(PanelStyle::RoughStonework)
		.openings(Openings::new())
		.build();
		Self { bounds, shell }
	}
}

impl BuildingFootprint for OldCityMarketTerrace {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

impl BuildingComponents for OldCityMarketTerrace {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.shell.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}
}

/// One graph site with a common platform and several independently posed stalls.
#[derive(Debug, Clone, PartialEq)]
pub struct OldCityMarketSite {
	pub position: Vec2,
	pub elevation: f32,
	pub tier: OldCityMarketTier,
	pub terrace: PlacedBuilding<OldCityMarketTerrace>,
	pub buildings: Vec<ShepherdsVillageBuilding>,
}

/// Graded market lane connecting two site terraces.
#[derive(Debug, Clone, PartialEq)]
pub struct OldCityMarketCorridor {
	pub path: Vec<Vec2>,
	pub levels: Vec<f32>,
	pub half_width: f32,
}

/// A market whose stall clusters are joined by a deterministic lane graph.
pub type OldCityMarket = ConnectedDevelopment<OldCityMarketSite, OldCityMarketCorridor>;

impl ConnectedDevelopment<OldCityMarketSite, OldCityMarketCorridor> {
	pub fn buildings(&self) -> impl Iterator<Item = &ShepherdsVillageBuilding> {
		self.nodes.iter().flat_map(|site| site.buildings.iter())
	}

	pub fn terraces(&self) -> impl Iterator<Item = &PlacedBuilding<OldCityMarketTerrace>> {
		self.nodes.iter().map(|site| &site.terrace)
	}

	pub fn corridors(&self) -> impl Iterator<Item = &OldCityMarketCorridor> {
		self.edges.iter().map(|edge| &edge.payload)
	}

	pub fn stall_count(&self) -> usize {
		self.nodes.iter().map(|site| site.buildings.len()).sum()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn terrace_owns_a_renderable_shared_footprint() -> anyhow::Result<()> {
		let terrace = OldCityMarketTerrace::new(Vec2::new(12.0, 18.0), Vec2::new(30.0, 24.0), 7.0);
		let footprint = terrace
			.footprint_rects()
			.into_iter()
			.next()
			.ok_or_else(|| anyhow::anyhow!("terrace should have a footprint"))?;
		assert_eq!(footprint.min, Vec2::new(-3.0, 6.0));
		assert_eq!(footprint.max, Vec2::new(27.0, 30.0));
		assert!(!terrace.panel_nodes_for_level(LodSceneLevel::High).is_empty());
		Ok(())
	}
}
