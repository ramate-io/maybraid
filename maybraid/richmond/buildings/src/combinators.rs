use bevy_math::bounding::Aabb3d;

pub struct Confines {
	pub bounds: Aabb3d,
	/// Horizontal roll which is applied to the primitives in the box after the fact.
	///
	/// The lower order builder doesn't care about this.
	pub roll: f32,
	/// The openings that the box wants in box local space.
	///
	/// In other words, the higher-order type has to place these with the appropriate roll applied.
	pub openings: Vec<Opening>,
}

/// Gives the box regions that are safe to fill with content.
///
/// The canonical concept is that these are boxes placed on-top of some existing
/// content in a way that is safe to fill.
///
/// This can be used for subdividing a neighborhood, stacking storeys, determining safe regions in a bedroom, etc.
pub trait Confines {
	/// Returns the safe authoring confines physically within the current type.
	fn confines_within(&self) -> Vec<Confines>;

	/// Returns the safe authoring confines physically atop the current type.
	///
	/// Usually, a higher-order type asks for this from a lower-order type atop which it would like to stack something.
	fn confines_atop(&self) -> Vec<Confines>;
}

/// Fits a type to a box given a seed value.
///
/// For non-noisy types, the seed value is typically ignored.
pub trait FitsBox: Sized {
	fn fit_to_box(seed: f32, confines: &Confines) -> Option<Self>;
}
