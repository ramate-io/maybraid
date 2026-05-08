//! Shared **SDF primitives** for Maybraid vegetation / Chico ([RFC-183](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation)).
//!
//! This crate layers reusable geometry on top of workspace [`sdf`] ([`sdf::Sdf`]):
//!
//! - [`TaperedCylinder`](crate::cylinder::TaperedCylinder) — tapered capped cylinder (stick / trunk segment).
//! - [`CrookCylinder`](crate::crook_cylinder::CrookCylinder) — tapered segment with smooth sinusoidal centerline ([RFC-183 §3.1.1.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/02-crook-cylinder/README.md), [#211](https://github.com/ramate-io/maybraid/issues/211)).
//! - [`NoisySurface`](crate::noisy::NoisySurface) — displacement from [`NoiseParams`](procedural_common::NoiseParams) via FastNoise Lite.
//! - [`NoisyCylinder`](crate::noisy::NoisyCylinder) / [`NoisyCrookCylinder`](crate::noisy::NoisyCrookCylinder) — noisy tapered / crook cylinders ([#210](https://github.com/ramate-io/maybraid/issues/210), [#211](https://github.com/ramate-io/maybraid/issues/211)).
//!
//! Optional **`clap`** / **`serde`** features add CLI and serialization derives on geometry and noise params.

pub use sdf;

pub use procedural_common::{FromScalarNoise, NoiseConfig, NoiseParams, sdf_band_margin};

pub mod crook_cylinder;
pub mod cylinder;
pub mod noisy;

pub use crook_cylinder::CrookCylinder;
pub use cylinder::TaperedCylinder;
pub use noisy::{NoisyCrookCylinder, NoisyCylinder, NoisySurface, UnitCylinderNoiseParams};
