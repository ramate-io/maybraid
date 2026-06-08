pub mod plugin;

use chico_groves::braid_grass_preview_cells;

use crate::render::RenderBraidGrass;

use super::CellRenderHelper;

impl CellRenderHelper<RenderBraidGrass> {
	pub fn configured_braid_grass(&self) -> RenderBraidGrass {
		let mut grass = self.render.inner.clone();
		grass.cells = braid_grass_preview_cells(self.cells_per_axis);
		grass
	}
}

/// Renders a Braid Grass grove into the scene.
pub type BraidGrassRenderHelper = CellRenderHelper<RenderBraidGrass>;
