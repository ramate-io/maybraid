pub mod plugin;

use bevy::prelude::Vec3;
use chico_groves::GroveExtent;

use crate::render::RenderTropicalTufts;

use super::CellRenderHelper;

impl CellRenderHelper<RenderTropicalTufts> {
	pub fn configured_tropical_tufts(&self) -> RenderTropicalTufts {
		let mut tufts = self.render.inner.clone();
		let cell_extent = tufts.grove.cell_extent_xz;
		let span = self.grove_extent_xz.max(cell_extent.x).max(cell_extent.y);
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		tufts.extent = extent;
		tufts
	}
}

/// Renders a Tropical Tufts grove into the scene.
pub type TropicalTuftsRenderHelper = CellRenderHelper<RenderTropicalTufts>;
