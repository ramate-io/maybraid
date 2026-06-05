//! Procedural primitives shared across Maybraid (noise, fields, …).

pub mod args;
pub mod distributions;
pub mod mu;
pub mod noise;

pub use args::{
	parse_count_pair, parse_unit_range, parse_usize_range, CountPair, UnitRange, UsizeRange,
};
pub use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
pub use mu::{sdf_band_margin, NUMERIC_SURFACE_EPSILON};
pub use noise::{
	noise_params_from_scalar_str, FromScalarNoise, NoiseConfig, NoiseParams, SetNoiseParams,
};
