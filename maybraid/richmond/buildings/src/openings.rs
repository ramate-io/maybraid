use bevy_math::bounding::Aabb3d;
use std::collections::HashMap;

pub struct Opening {
	pub bounds: Aabb3d,
}

/// A labeled spatial index on openings.
///
/// This is used with many types to to determine where they should and should not write geometry.
///
/// It is supposed to be a spatial index. The HashMap is a placeholder.
pub struct Openings {
	pub openings: HashMap<String, Opening>,
}

/// A label for an opening.
pub struct Label(String);

impl Openings {
	pub fn new() -> Self {
		Self { openings: HashMap::new() }
	}

	pub fn intersecting_openings(
		&self,
		bounds: Aabb3d,
		labels: Option<Vec<Label>>,
	) -> Vec<&Opening> {
		todo!()
	}

	/// Finds an opening that is closest and most similar in size to the given bounds.
	///
	/// This is useful for finding potential connecting points.
	pub fn best_fit_opening(&self, bounds: Aabb3d, labels: Option<Vec<Label>>) -> Option<&Opening> {
		todo!()
	}
}
