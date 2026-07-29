//! Procedural primitives shared across Maybraid (noise, fields, …).

pub mod args;
pub mod distributions;
pub mod hysteresis;
pub mod mu;
pub mod noise;
pub mod path;

pub use args::{
	parse_count_pair, parse_u32_range, parse_unit_range, parse_usize_range, CountPair, U32Range,
	UnitRange, UsizeRange,
};
pub use distributions::bucket_throw::{Bucket, BucketThrow, TypedBucketThrow};
pub use distributions::{perturb_weights, FirstFitIndices, MIN_BUCKET_WEIGHT};
pub use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};
pub use hysteresis::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};
pub use mu::{sdf_band_margin, NUMERIC_SURFACE_EPSILON};
pub use noise::{
	noise_params_from_scalar_str, BuildWithNoise, FromScalarNoise, NoiseConfig, NoiseParams,
};
pub use path::{noisy_path, AllowedAngles, NoisyPathParams, StepLenRange};
