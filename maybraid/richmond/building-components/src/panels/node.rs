//! Panel IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::lod_ref::LodRef;
use lod::{LodLazyPending, SceneChunk};
use material_ref::{MaterialRef, MaterialRefRoot, PropagateToDescendants};
use scene_ref::MirrorAxis;

use crate::assets::AssetPath;
use crate::layer::Layers;
use crate::lod_band::{placement_bounds, warm_mesh_lod_culls};
use crate::panels::geometry::{PanelGeometry, Rectangle, RightTriangle};
use crate::panels::lod::{
	panel_scene_ref_for_level, PanelLodProbe, PANEL_ULTRA_LOW_RECTANGLE,
	PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
};
use crate::panels::style::PanelStyle;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children, with_pose};

/// Authoring IR for a shared panel feature (rectangle / triangle tessellation).
///
/// [`Self::style`] picks the kit GLB path. [`Self::material`] is an optional
/// shader look ([`MaterialRef`]) stamped onto that kit after spawn.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct PanelNode {
	pub style: PanelStyle,
	pub geometry: PanelGeometry,
	pub placement: Placement,
	pub material: Option<MaterialRef>,
}

impl PanelNode {
	pub fn new(style: PanelStyle, geometry: PanelGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, material: None }
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	pub fn rough_stone(geometry: PanelGeometry, placement: Placement) -> Self {
		Self::new(PanelStyle::RoughStonework, geometry, placement)
	}

	pub fn shepherds_thatch(geometry: PanelGeometry, placement: Placement) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, geometry, placement)
	}

	fn probe(&self) -> PanelLodProbe {
		PanelLodProbe::from_placement(&self.placement)
	}

	fn content_for_level(&self, level: LodSceneLevel) -> impl Scene + 'static {
		let material = self.material.clone();
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.flatten(self.style.kit_caps())
			.into_iter()
			.filter_map(|piece| {
				let transform = pose(self.placement) * pose(piece.placement);
				match piece.geom {
					PanelGeometry::Rectangle(Rectangle) => {
						let (high, mid, low) = self.style.rectangle_lod()?;
						Some(Box::new(with_pose(
							transform,
							lod_quad_scene(
								high,
								mid,
								low,
								PANEL_ULTRA_LOW_RECTANGLE,
								level,
								None,
								material.clone(),
							),
						)) as Box<dyn Scene>)
					}
					PanelGeometry::RightTriangle(RightTriangle { mirror }) => {
						let (high, mid, low) = self.style.right_triangle_lod()?;
						Some(Box::new(with_pose(
							transform,
							lod_quad_scene(
								high,
								mid,
								low,
								PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
								level,
								mirror,
								material.clone(),
							),
						)) as Box<dyn Scene>)
					}
					_ => None,
				}
			})
			.collect();
		scene_children(children)
	}
}

impl Layers<PanelNode> {
	/// Stamp a shader look onto every panel, leaving kit [`PanelStyle`] unchanged.
	pub fn with_material(self, material: MaterialRef) -> Self {
		self.map(|node| node.with_material(material.clone()))
	}
}

fn lod_quad_scene(
	high: AssetPath,
	mid: AssetPath,
	low: AssetPath,
	ultra_low: AssetPath,
	level: LodSceneLevel,
	mirror: Option<MirrorAxis>,
	material: Option<MaterialRef>,
) -> Box<dyn Scene> {
	let scene = panel_scene_ref_for_level(
		high.scene_ref().with_mirror(mirror),
		mid.scene_ref().with_mirror(mirror),
		low.scene_ref().with_mirror(mirror),
		ultra_low.scene_ref().with_mirror(mirror),
		level,
	);
	material_kit_scene(scene, material)
}

fn material_kit_scene(
	scene: impl Scene + 'static,
	material: Option<MaterialRef>,
) -> Box<dyn Scene> {
	match material {
		Some(material) => Box::new((
			bsn! {
				template_value(MaterialRefRoot(material))
				PropagateToDescendants
				LodLazyPending
			},
			scene,
		)),
		None => Box::new(scene),
	}
}

impl LodScene for PanelNode {
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

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::panels::geometry::PanelGeometry;

	#[test]
	fn with_material_stamps_ref_without_changing_style() {
		let node =
			PanelNode::rough_stone(PanelGeometry::Rectangle(Rectangle), Placement::default())
				.with_material(MaterialRef::named("stucco"));
		assert_eq!(node.style, PanelStyle::RoughStonework);
		assert_eq!(
			node.material.as_ref().map(|m| &m.name),
			Some(&material_ref::MaterialId::named("stucco"))
		);
	}
}
