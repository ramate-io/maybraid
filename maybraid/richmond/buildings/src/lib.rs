//! A common constraint representation for building generation.
//!
//! Often, for complicated patterns, authors will use a combination of
//! this representation and direct access to parent types to build
//! the authored type.
//!
//! At the same time, common constructions built only requiring this representation
//! can be reused within authored types.
//!
//! The cells are rectangular prisms, describing authoring bounds.
//! The authored types do not need, however, to author strictly rectangular geometry.

use bevy_math::{
	bounding::{Aabb2d, Aabb3d},
	Vec3,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BoundaryThicknessEntry {
	/// The fallback thickness for the boundary (common when the boundary is one thickness)
	fallback: f32,
	/// The thickness for each sub-region of the boundary.
	sub_regions: HashMap<Aabb2d, f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoundaryOwnershipStatus {
	/// The boundary is owned by the cell.
	///
	/// This is not typically stored, it is implied by the absence of other values
	#[default]
	Cell,
	/// The boundary is owned by a parent of the cell.
	///
	/// Typically, routines will not write on this boundary, except for ornamental details.
	Parent,
	/// The boundary is owned by a sibling of the cell.
	///
	/// Typically, routines will not write on this boundary, except for ornamental details.
	Sibling,
}

/// The entry for one boundary segment of the cell.
///
/// Whole and SubRegions are intentionally mutually exclusive at the type level.
/// SubRegions should probably be behaviorally exclusive, i.e.,
/// disallow setting an overlapping sub-region--perhaps with an escape hatch to overwrite.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoundaryOwnershipEntry {
	/// The whole of the specified boundary segment, i.e. one side of the cell's aabb.
	#[default]
	Whole(BoundaryOwnershipStatus),
	/// A sub-region of the specified boundary segment, i.e. a part of one side of the cell's aabb.
	///
	/// Not sure if the Aabb2d should be world space or local space.
	/// World space might be more generally convenient, especially when passing down to children without normalizing.
	SubRegions(HashMap<Aabb2d, BoundaryOwnershipStatus>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CirculationRequestStatus {
	/// The circulation request has already been fulfilled in the hierarchy.
	///
	/// This is not typically stored; it is implied by the absence of other values.
	#[default]
	Satisfied,
	/// The hierarchy has determined that some kind of circulation element is required at this boundary region.
	Required,
	/// The hierarchy has determined that some kind of circulation element is desired at this boundary region--if the child can make it work.
	Desired,
	/// The hierarchy has suggested this could be a good place for a circulation element, but it is not required or desired.
	Optional,
}

/// The description of the total circulation requirements for a region on the boundary.
///
/// Unlike ownership, circulation requirements are not assumed exclusive,
/// though they often have to be be combined with the ownership table to determine the final requirements.
/// In fact, we should probably have a safe API on [CellConstraints] to prevent entering circulation requirements
/// on boundary regions which are not owned by the cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CirculationEntry(HashMap<Aabb2d, Vec<CirculationRequestStatus>>);

/// A sample of the incoming boundary geometry before the joint point.
///
/// A series of three dimensional points, probably normalized to boundary space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreJointSweep(Vec<Vec3>);

/// The coordinate of a joint on the boundary.
///
/// Interestingly, joints will typically fall on the edge of ownership boundaries.
/// We should probably provide a behavioral unificaiton path for this case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JointCoordinate {
	/// The distance along the boundary segment.
	pub t: f32,
	/// The height up the boundary segment.
	pub h: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JointEntry(HashMap<JointCoordinate, PreJointSweep>);

/// The complete boundary table for the cell.
///
/// Often, a lower order cell will subset boundary table from its parent,
/// add its own boundary table, and then pass it down to its children.
///
/// More complicated scenarios are better handled by
/// the child simply takig a reference to parents, grandparent, etc.
/// at generation time. But, this can be a useful common parameterization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellBoundaryTable<T: Clone> {
	/// Bevy +Y
	pub top: Option<T>,
	/// Bevy -Y
	pub bottom: Option<T>,
	/// Bevy -X (double-check this)
	pub left: Option<T>,
	/// Bevy +X
	pub right: Option<T>,
	/// Bevy -Z (double-check this)
	pub front: Option<T>,
	/// Bevy +Z
	pub back: Option<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CellConstraints {
	/// The spatial constraints of the cell, within these bounds, the cell has write authority.
	pub aabb: Aabb3d,
	/// The boundary thickness for the cell.
	pub boundary_thickness: CellBoundaryTable<BoundaryThicknessEntry>,
	/// The boundary ownership map for the cell.
	pub boundary_ownership: CellBoundaryTable<BoundaryOwnershipEntry>,
	/// The circulation requirements for the cell.
	pub circulation: CellBoundaryTable<CirculationEntry>,
	/// The joints for the cell.
	pub joints: CellBoundaryTable<JointEntry>,
}
