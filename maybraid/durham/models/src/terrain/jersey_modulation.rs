//! Adapter from jersey / marazion hydro height ops onto durham [`ElevationModulation`].

use crate::terrain::sdf::{ElevationModulation, TerrainSdf};
use jersey_terrain_stamps::JerseyModulation;
use marazion_watersheds::{CorrectionStage, PreparedHydroComplex};

/// One elevation op in the final terrain stack.
///
/// Watershed correction is staged: carve → rim → apron (not a closed leaf op).
#[derive(Debug, Clone)]
pub enum ComposedElevationOp {
	Jersey(JerseyModulation),
	WatershedCarve(PreparedHydroComplex),
	WatershedRim(PreparedHydroComplex),
	WatershedApron(PreparedHydroComplex),
}

impl ComposedElevationOp {
	pub fn modify_elevation_xz(&self, elevation: f32, x: f32, z: f32) -> f32 {
		match self {
			Self::Jersey(m) => JerseyModulation::modify_elevation(m, elevation, x, z),
			Self::WatershedCarve(h) => h.apply_stage(CorrectionStage::Carve, elevation, x, z),
			Self::WatershedRim(h) => h.apply_stage(CorrectionStage::Rim, elevation, x, z),
			Self::WatershedApron(h) => h.apply_stage(CorrectionStage::Apron, elevation, x, z),
		}
	}
}

impl ElevationModulation for JerseyModulation {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		JerseyModulation::modify_elevation(self, elevation, x, z)
	}
}

impl ElevationModulation for ComposedElevationOp {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		self.modify_elevation_xz(elevation, x, z)
	}
}

impl ElevationModulation for PreparedHydroComplex {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		// Full stack only for ad-hoc sampling; terrain uses staged ops.
		PreparedHydroComplex::modify_elevation(self, elevation, x, z)
	}
}
