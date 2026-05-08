//! Shared **SDF primitives** for Maybraid vegetation / Chico ([RFC-183](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation)).
//!
//! This crate layers reusable geometry on top of workspace [`sdf`] ([`sdf::Sdf`]):
//!
//! - [`TaperedCylinder`](crate::cylinder::TaperedCylinder) — tapered capped cylinder (stick / trunk segment).
//! - [`NoisySurface`](crate::noisy::NoisySurface) — displacement from [`NoiseParams`](procedural_common::NoiseParams) via FastNoise Lite.
//! - [`NoisyCylinder`](crate::noisy::NoisyCylinder) — convenience alias for a noisy tapered cylinder ([#210](https://github.com/ramate-io/maybraid/issues/210)).
//!
//! Optional **`clap`** / **`serde`** features add CLI and serialization derives on geometry and noise params.

pub use sdf;

pub use procedural_common::{FromScalarNoise, NoiseConfig, NoiseParams, sdf_band_margin};

pub mod cylinder;
pub mod noisy;

pub use cylinder::TaperedCylinder;
pub use noisy::{NoisyCylinder, NoisySurface};
