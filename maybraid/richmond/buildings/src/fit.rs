//! Shared fitting contract: place a type inside [`Confines`] from spatial noise.
//!
//! Most complete fitting types follow **Parameterized → FloorPlan → Full\***:
//! sample knobs from noise, emit structure + residual [`FillableRegions`], then
//! fill those regions (or leave them for a later typology).
//!
//! **Towering:** sample a FloorPlan once, then rebuild upper levels with
//! `from_parameterized` + Y-lifted confines so the plan stays identical. When
//! stacking the *same* storey type, prefer that path and ignore [`FillableRegions::atop`]
//! so irregular footprints are not re-derived per level.
//!
//! Axis-aligned bound helpers used while fitting live here alongside [`Confines`]
//! ([`aabb_near_plane`], [`aabb_xz_near_eq`], [`aabb_xz_overlap_area`], …).

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
use bevy_math::{Vec2, Vec3};
use procedural_common::NoiseParams;

use crate::openings::Openings;

/// Describes the confines in which the object must fit.
#[derive(Debug, Clone, PartialEq)]
pub struct Confines {
	/// The bounding box of the confines.
	pub bounds: Aabb3d,
	/// The roll of the confines.
	///
	/// Typically consumed by higher-order types to rotate all primitives emitted.
	pub roll: f32,
	/// Openings required on (or reserved within) the confines.
	pub openings: Openings,
}

impl Confines {
	pub fn new(bounds: Aabb3d, roll: f32, openings: Openings) -> Self {
		Self {
			bounds,
			roll,
			openings,
		}
	}

	pub fn from_bounds(bounds: Aabb3d) -> Self {
		Self::new(bounds, 0.0, Openings::new())
	}

	/// World-space center of [`Self::bounds`] (spatial noise sample point).
	pub fn center(&self) -> Vec3 {
		self.bounds.center().into()
	}

	/// Plan-space center \((x, z)\) of [`Self::bounds`].
	pub fn center_xz(&self) -> Vec2 {
		aabb_xz_center(&self.bounds)
	}

	/// Footprint extents \((x, z)\) of [`Self::bounds`].
	pub fn footprint(&self) -> Vec2 {
		aabb_xz_extent(&self.bounds)
	}
}

/// True when the closed interval `[lo, hi]` reaches within `tol` of `plane`.
#[inline]
pub fn aabb_near_plane(lo: f32, hi: f32, plane: f32, tol: f32) -> bool {
	lo <= plane + tol && hi >= plane - tol
}

/// Plan-space center \((x, z)\) of an AABB.
#[inline]
pub fn aabb_xz_center(a: &Aabb3d) -> Vec2 {
	let min = Vec3::from(a.min);
	let max = Vec3::from(a.max);
	Vec2::new((min.x + max.x) * 0.5, (min.z + max.z) * 0.5)
}

/// Plan footprint size \((x, z)\) of an AABB.
#[inline]
pub fn aabb_xz_extent(a: &Aabb3d) -> Vec2 {
	let min = Vec3::from(a.min);
	let max = Vec3::from(a.max);
	Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0))
}

/// Plan footprint area \(x \cdot z\) of an AABB.
#[inline]
pub fn aabb_xz_area(a: &Aabb3d) -> f32 {
	let e = aabb_xz_extent(a);
	e.x * e.y
}

/// True when two AABBs share nearly the same XZ footprint (Y ignored).
#[inline]
pub fn aabb_xz_near_eq(a: &Aabb3d, b: &Aabb3d, eps: f32) -> bool {
	let amin = Vec3::from(a.min);
	let amax = Vec3::from(a.max);
	let bmin = Vec3::from(b.min);
	let bmax = Vec3::from(b.max);
	(amin.x - bmin.x).abs() < eps
		&& (amin.z - bmin.z).abs() < eps
		&& (amax.x - bmax.x).abs() < eps
		&& (amax.z - bmax.z).abs() < eps
}

/// Overlap area between an AABB’s XZ footprint and a plan [`Aabb2d`]
/// (\(x → X\), \(y → Z\)).
#[inline]
pub fn aabb_xz_overlap_area(a: &Aabb3d, region: &Aabb2d) -> f32 {
	let amin = Vec3::from(a.min);
	let amax = Vec3::from(a.max);
	let x0 = amin.x.max(region.min.x);
	let x1 = amax.x.min(region.max.x);
	let z0 = amin.z.max(region.min.y);
	let z1 = amax.z.min(region.max.y);
	(x1 - x0).max(0.0) * (z1 - z0).max(0.0)
}

/// Vertical box atop the confines on which other types can stack.
///
/// [`Self::bounds`] is the plan footprint: `Aabb2d` \(x → world X, y → world Z\).
#[derive(Debug, Clone, PartialEq)]
pub struct StackRegion {
	/// Plan footprint (\(x, z\) as [`Aabb2d`] \(x, y\)).
	pub bounds: Aabb2d,
	/// Height of the stack region (world Y extent available above the storey).
	pub height: f32,
	/// Roll of the stack region.
	pub roll: f32,
	/// Openings required when stacking on this region.
	pub openings: Openings,
}

/// Residual regions after a successful fit.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FillableRegions {
	/// Regions that can be filled within.
	///
	/// Used for composing so livable types need not fully describe internal
	/// confines (e.g. FloorPlan → FullStorey). The FloorPlan emits structure and
	/// `within`; Full\* fills those confines with specific types. The same
	/// FloorPlan can back other Full\* variants.
	pub within: Vec<Confines>,
	/// Regions on which we can stack.
	///
	/// Typically used when transitioning between tower and storey types. When
	/// using the same storey, prefer towering via a copied floor plan and ignore
	/// this so irregular geometry stays stable.
	pub atop: Vec<StackRegion>,
}

impl FillableRegions {
	pub fn empty() -> Self {
		Self::default()
	}
}

/// Failure modes for [`Fit::fit_to_confines`].
///
/// Soft volume rejects use [`FitError::TooSmall`] so a higher-order chooser can
/// try another typology. Hard invariant breaks use [`FitError::InvalidConfines`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FitError {
	/// Confines are too small for this type's minimum extents.
	TooSmall {
		/// Short reason (e.g. `"footprint"`, `"height"`).
		reason: &'static str,
	},
	/// Confines are malformed or otherwise unusable.
	InvalidConfines {
		reason: &'static str,
	},
}

impl std::fmt::Display for FitError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::TooSmall { reason } => write!(f, "too small: {reason}"),
			Self::InvalidConfines { reason } => write!(f, "invalid confines: {reason}"),
		}
	}
}

impl std::error::Error for FitError {}

/// Fit a type into confines from spatial noise.
///
/// Implementors should sample with [`NoiseParams`] → [`procedural_common::NoiseConfig`]
/// at the confines center (distinct `w` salt lanes per decision). Soft rejects
/// return [`FitError::TooSmall`].
pub trait Fit: Sized {
	/// Fit `Self` to `confines` using spatial `noise`.
	///
	/// On success, returns the fitted object and residual fillable regions.
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError>;
}
