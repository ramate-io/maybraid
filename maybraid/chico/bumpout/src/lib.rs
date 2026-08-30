//! Terrain-mesh bump outs for Chico ground cover and distant canopy mass.
//!
//! A [`BumpOut`] and ordinary terrain presenter may carry the same
//! [`terrain_chunk_ref::TerrainChunkRef`]. Lazy terrain fulfillment gives both entities the same
//! mesh handle, while [`BumpOutMaterial`] performs cell-profile blending, vertical displacement,
//! and fragment dropout in the shader.

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
};

pub const CHICO_BUMP_OUT_MATERIAL: &str = "chico_bump_out";
pub const DENSITY_PARAMETER: &str = "neighborhood_density";
pub const BITE_SIZE_PARAMETER: &str = "neighborhood_bite_size";
pub const BITE_SIZE_DEVIATION_PARAMETER: &str = "neighborhood_bite_size_deviation";
pub const AVERAGE_HEIGHT_PARAMETER: &str = "neighborhood_average_height";
pub const HEIGHT_DEVIATION_PARAMETER: &str = "neighborhood_height_deviation";
pub const STYLE_PARAMETER: &str = "bump_out_style";

/// Registers the bump-out shader and standalone deferred material resolver.
pub struct ChicoBumpOutPlugin;

impl Plugin for ChicoBumpOutPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((BumpOutMaterialPlugin, BumpOutMaterialRefPlugin));
	}
}
