use crate::Opening;
use bevy_math::bounding::Aabb2d;

/// Describes the confines in which the object must fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Confines {
	/// The bounding box of the confines.
	pub bounds: Aabb3d,
	/// The roll of the confines.
	///
	/// Typically, this is only consumed by higher-order types to rotate all primitives emitted.
	pub roll: f32,
	/// The openings required on the confines.
	pub openings: Vec<Opening>,
}

/// Describes vertical boxes atop the confines on which other types can stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StackRegion {
	/// The bounding box of the stack region.
	pub bounds: Aabb2d,
	/// The height of the bounding box of the stack region.
	pub height: f32,
	/// The roll of the bounding box of the stack region.
	pub roll: f32,
	/// Any openings required when stacking on this region.
	pub openings: Vec<Opening>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FillableRegions {
	/// The regions that can be filled within.
	///
	/// This is typically used for general composing so
	/// that livable types don't have to completely describe their internal confines
	/// and can be reused, e.g., a FloorPlan -> FullStorey pattern.
	///
	/// The FloorPlan describes the basic structure and emits confines.
	/// The FullStorey fills these confines with specific types.
	/// The FloorPlan can then be reused for other FullStorey types.
	pub within: Vec<Confines>,
	/// The regions on which we can stack.
	///
	/// Typically, this is used when you are transitioning between tower and storey types.
	/// When you are using the same storey, it is typically best to tower and ignore this
	/// to better handle irregular geometry.
	pub atop: Vec<StackRegion>,
}

/// The Fit trait was first used with storeys, but is meant for general fitting purposes.
pub trait Fit {
	/// Fits the object to the confines for a given seed. Outputs the object and the remaining fillable regions.
	///
	/// When you are towering, you will often generate the first level and then copy its floor plan and confines upwards,
	/// matching the first shape.
	///
	/// Note, you do not have access to any context besides the confines and a seed.
	/// This is where noisy generation of a type typically occurs.
	/// It is common to have a parameterized backing type
	/// the values on which we generate to choose the type.
	/// Hence, the most common pattern for a complete fitting type is...
	/// Parameterized -> FloorPlan -> Full
	fn fit_to_confines(confines: &Confines, seed: f32)
		-> Result<(Self, FillableRegions), FitError>;
}
