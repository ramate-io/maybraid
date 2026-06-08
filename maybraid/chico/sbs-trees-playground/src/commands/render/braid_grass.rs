pub mod plugin;

use bevy::prelude::Vec3;
use chico_groves::{braid_grass::BraidGrassDefinition, GroveExtent};

use crate::render::RenderBraidGrass;

use super::CellRenderHelper;

impl CellRenderHelper<RenderBraidGrass> {
	pub fn configured_braid_grass(&self) -> RenderBraidGrass {
		let mut grass = self.render.inner.clone();
		let count = self.cells_per_axis.max(1);
		let span = count as f32 * BraidGrassDefinition::cell_extent_xz_default();
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass.extent = extent;
		grass.cells_per_axis = count;
		grass
	}
}

/// Renders a Braid Grass grove into the scene.
pub type BraidGrassRenderHelper = CellRenderHelper<RenderBraidGrass>;
