pub mod plugin;

use bevy::prelude::Vec3;
use chico_groves::GroveExtent;

use crate::render::RenderBraidGrass;

use super::CellRenderHelper;

impl CellRenderHelper<RenderBraidGrass> {
	pub fn configured_braid_grass(&self) -> RenderBraidGrass {
		let mut grass = self.render.inner.clone();
		let cell_extent = grass.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell_extent.x).max(cell_extent.y);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass.extent = extent;
		grass
	}
}

/// Renders a Braid Grass grove into the scene.
pub type BraidGrassRenderHelper = CellRenderHelper<RenderBraidGrass>;
