//! Procedural primitives shared across Maybraid (noise, fields, …).

pub mod args;
pub mod bounds;
pub mod distributions;
pub mod hysteresis;
pub mod mu;
pub mod noise;
pub mod packing;
pub mod path;

pub use args::{
	parse_count_pair, parse_u32_range, parse_unit_range, parse_usize_range, CountPair, U32Range,
	UnitRange, UsizeRange,
};
pub use bounds::{
	aabb2_area, aabb3_to_plan, clamp_min_size2, grow_aabb2, grow_aabb2_pair, inflate_aabb2,
	inset_aabb2, intersects_aabb2, max_empty_aabb3_plan, max_empty_rect2, max_empty_rect2_by,
	max_empty_rect2_with_clearance, plan_to_aabb3, touches_aabb2, PlanAxes,
};
pub use packing::{
	band_clear_of, bipartition_aabb2_by_area, contacts_opening_face, face_band,
	free_segments_on_face, grow_aabb2_pair_toward_areas, grow_aabb2_toward_area,
	longest_free_face_segment, pack_optional_face_bands, passage_opening_face,
	seed_from_free_opening_face, seed_from_opening_face, shared_opening_border_len,
	OptionalFaceBand, PlanOpeningFace,
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
