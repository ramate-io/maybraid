use bevy_math::bounding::Aabb3d;
use std::collections::HashMap;

/// A label for an opening.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Label(String);

/// An opening is a 3D bounding box with a label.
///
/// It is consider a unique identifier. Two openings with the same bounds and label are considered the same opening.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Opening {
	pub bounds: Aabb3d,
	/// The label of the opening.
	///
	/// This carries a bit of dynamic information that can help downstream systems figure out
	/// how to use the the opening.
	///
	/// Currently, this sits on the opening object itself for simplicity, i.e.,
	/// avoiding IDing openings and mapping them to labels.
	pub label: Option<Label>,
}

/// A spatial index of labeled openings.
///
/// This is used with many types to to determine where they should and should not write geometry.
///
/// It is supposed to be a spatial index. The HashMap is a placeholder.
///
/// Typically, this functions both as a plan and as a record.
/// However, exact geometry matches, still need a reference to the type.
pub struct Openings {
	pub openings: HashMap<String, Opening>,
}

impl Openings {
	pub fn new() -> Self {
		Self { openings: HashMap::new() }
	}

	/// Finds all openings that intersect with the given bounds.
	///
	/// Note, we don't filter by labels here. This is a broad-phase.
	/// For most queries, it should be cheaper to check all openings, then filter by desired labels as opposed to filtering by labels first.
	pub fn intersecting_openings(&self, bounds: Aabb3d) -> Vec<&Opening> {
		todo!()
	}

	/// Finds an opening that is closest and most similar in size to the given bounds.
	///
	/// This is useful for finding potential connecting points.
	pub fn best_fit_opening(&self, fit_requirements: &OpeningFit) -> Option<&Opening> {
		todo!()
	}

	/// Finds a mapped opening that is closest and most similar in size to the given bounds.
	pub fn best_fit_mapped_opening(
		&self,
		maps_openings: &impl MapsOpenings,
		fit_requirements: &OpeningFit,
	) -> Option<MappedOpening> {
		self.best_fit_opening(fit_requirements)
			.and_then(|opening| maps_openings.map_opening(opening))
	}
}

/// Cost matrix for the distance between two Aabb3d bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DistanceCost {
	pub front_lower_left: f32,
	pub front_lower_right: f32,
	pub front_upper_left: f32,
	pub front_upper_right: f32,
	pub back_lower_left: f32,
	pub back_lower_right: f32,
	pub back_upper_left: f32,
	pub back_upper_right: f32,
}

/// Cost matrix for the scale of two Aabb3d bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScaleCost(Vec3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelCost {
	Filter,
	Cost(f32),
}

/// The cost matrix for labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LabelCosts(HashMap<Label, LabelCost>);

/// An ideal fit cost descriptor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IdealFit {
	/// The ideal opening bounds.
	pub bounds: Aabb3d,
	/// The cost per pair-wise distance to each point on the ideal bounds.
	pub distance_cost: DistanceCost,
	/// The cost of the scale of the opening.
	pub scale_cost: ScaleCost,
}

/// The cost descriptor for an opening fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OpeningFit {
	/// The opening intersects these bounds.
	pub bounds: Aabb3d,
	/// The cost of different labels for the opening, also functions as a filter.
	pub label_costs: LabelCosts,
	/// The ideal opening bounds.
	pub ideal_fit: Option<IdealFit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MappedOpeningQuad {
	pub lower_left: Vec3,
	pub lower_right: Vec3,
	pub upper_left: Vec3,
	pub upper_right: Vec3,
}

/// An opening mapped onto the geometry of the object.
///
/// Note that here we are
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MappedOpening {
	front_face: MappedOpeningQuad,
	back_face: MappedOpeningQuad,
}

/// A table storing the mapped openings for each opening.
///
/// Some types implementing `MapsOpenings` may choose
/// to store mapped openings at construction time, particularly for certain labels, e.g.,
/// Entryway.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MappedOpenings(HashMap<Opening, MappedOpening>);

pub trait MapsOpenings {
	/// Maps an Aabb3d opening onto the actual geometry of the object.
	///
	/// If the construction did not map the opening, it should typically return `None`
	/// instead of virtualizing where the opening would be.
	fn map_opening(&self, opening: &Opening) -> Option<MappedOpening>;
}
