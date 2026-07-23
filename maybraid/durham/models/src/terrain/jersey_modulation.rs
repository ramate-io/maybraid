//! Adapter from jersey / marazion hydro height ops onto durham [`ElevationModulation`].

use crate::terrain::sdf::{ElevationModulation, TerrainSdf};
use jersey_terrain_stamps::JerseyModulation;
use marazion_watersheds::HydrologyComplex;

/// One elevation op in the final terrain stack.
#[derive(Debug, Clone)]
pub enum ComposedElevationOp {
	Jersey(JerseyModulation),
	/// Indexed hydrology complex (internally carve → rim → apron).
	Hydrology(HydrologyComplex),
}

impl ComposedElevationOp {
	pub fn modify_elevation_xz(&self, elevation: f32, x: f32, z: f32) -> f32 {
		match self {
			Self::Jersey(m) => JerseyModulation::modify_elevation(m, elevation, x, z),
			Self::Hydrology(h) => HydrologyComplex::modify_elevation(h, elevation, x, z),
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

impl ElevationModulation for HydrologyComplex {
	fn modify_elevation(
		&self,
		_terrain: &TerrainSdf,
		elevation: f32,
		x: f32,
		z: f32,
		_index: usize,
	) -> f32 {
		HydrologyComplex::modify_elevation(self, elevation, x, z)
	}
}
