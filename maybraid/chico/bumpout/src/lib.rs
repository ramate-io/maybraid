//! Terrain-mesh bump outs for Chico ground cover and distant canopy mass.
//!
//! A [`BumpOut`] and ordinary terrain presenter may carry the same
//! [`terrain_chunk_ref::TerrainChunkRef`]. Lazy terrain fulfillment gives both entities the same
//! mesh handle, while [`BumpOutMaterial`] performs cell-profile blending, vertical displacement,
//! and fragment dropout in the shader.
//!
//! GPU uniforms are a shared raster/scalar pad. This shader reads density / bite / height
//! on channels 0–4 and style on scalars 0–6.

mod bump_out;
mod material;
mod neighborhood;

use bevy::prelude::{App, Plugin};

pub use bump_out::BumpOut;
pub use material::{
	BumpOutMaterial, BumpOutMaterialLib, BumpOutMaterialPlugin, BumpOutMaterialRefCache,
	BumpOutMaterialRefPlugin, BumpOutUniform,
};
pub use neighborhood::{
	BumpOutNeighborhood, BumpOutStyle, BUMP_OUT_NEIGHBORHOOD_SAMPLES, BUMP_OUT_NEIGHBORHOOD_WIDTH,
	RASTER_AVERAGE_HEIGHT, RASTER_BITE_SIZE, RASTER_BITE_SIZE_DEVIATION, RASTER_DENSITY,
	RASTER_HEIGHT_DEVIATION,
};

pub const CHICO_BUMP_OUT_MATERIAL: &str = "chico_bump_out";

/// Registers the bump-out shader and standalone deferred material resolver.
pub struct ChicoBumpOutPlugin;

impl Plugin for ChicoBumpOutPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((BumpOutMaterialPlugin, BumpOutMaterialRefPlugin));
	}
}
