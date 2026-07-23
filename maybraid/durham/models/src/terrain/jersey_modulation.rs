//! Adapter from jersey / marazion hydro height ops onto durham [`ElevationModulation`].

use crate::terrain::sdf::{ElevationModulation, TerrainSdf};
use jersey_terrain_stamps::JerseyModulation;
use marazion_watersheds::PreparedHydroComplex;

/// One elevation op in the final terrain stack: jersey landform or hydro complex.
#[derive(Debug, Clone)]
pub enum ComposedElevationOp {
	Jersey(JerseyModulation),
	Watershed(PreparedHydroComplex),
}

impl ComposedElevationOp {
	pub fn modify_elevation_xz(&self, elevation: f32, x: f32, z: f32) -> f32 {
		match self {
			Self::Jersey(m) => JerseyModulation::modify_elevation(m, elevation, x, z),
			Self::Watershed(h) => h.modify_elevation(elevation, x, z),
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
		PreparedHydroComplex::modify_elevation(self, elevation, x, z)
	}
}
