//! Shared **SDF primitives** for Maybraid vegetation / Chico ([RFC-183](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation)).
//!
//! This crate layers reusable geometry on top of workspace [`sdf`] ([`sdf::Sdf`]):
//!
//! - [`TaperedCylinder`](crate::cylinder::TaperedCylinder) — tapered capped cylinder (stick / trunk segment).
//! - [`NoisySurface`](crate::noisy::NoisySurface) — displacement via [`procedural_common::NoiseConfig::sample_3d_world`] and [`NoiseParams::domain_weights`](procedural_common::NoiseParams).
//! - [`NoisyCylinder`](crate::noisy::NoisyCylinder) — convenience alias for a noisy tapered cylinder ([#210](https://github.com/ramate-io/maybraid/issues/210)).
//!
//! - [`SdfCommonPrimitive`](crate::primitive::SdfCommonPrimitive) — unified enum + [`SdfCommonRenderItem`](crate::primitive::SdfCommonRenderItem) for rendering / playgrounds.

pub use sdf;

pub use procedural_common::{FromScalarNoise, NoiseConfig, NoiseParams, sdf_band_margin};

pub mod cylinder;
pub mod noisy;
pub mod primitive;

pub use cylinder::TaperedCylinder;
pub use noisy::{NoisyCylinder, NoisySurface};
pub use primitive::{SdfCommonPrimitive, SdfCommonRenderItem};
