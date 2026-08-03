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

use bevy_math::bounding::{Aabb2d, Aabb3d, BoundingVolume};
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
	pub fn center(&self) -> bevy_math::Vec3 {
		self.bounds.center().into()
	}
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
