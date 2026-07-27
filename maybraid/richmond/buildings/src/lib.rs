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

pub mod wizards_tower;

use bevy_math::{
	bounding::{Aabb2d, Aabb3d},
	Vec2, Vec3,
};

/// Epsilon for face-coincidence and containment checks.
const FACE_EPS: f32 = 1e-4;

/// A boundary sub-region paired with a value.
///
/// Region [`Aabb2d`] values are **boundary-local**:
/// - \(x\) (`t`) runs along the face's primary length in \([0, 1]\)
/// - \(y\) (`h`) runs along the face's secondary axis in \([0, 1]\)
///   (vertical for side faces; the other horizontal axis for top/bottom)
///
/// Uses a list rather than a map because [`Aabb2d`] is not `Eq`/`Hash` in Bevy.
pub type BoundaryRegionList<T> = Vec<(Aabb2d, T)>;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct BoundaryThicknessEntry {
	/// The fallback thickness for the boundary (common when the boundary is one thickness).
	pub fallback: f32,
	/// The thickness for each sub-region of the boundary.
	pub sub_regions: BoundaryRegionList<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BoundaryOwnershipStatus {
	/// The boundary is owned by the cell.
	///
	/// This is not typically stored; it is implied by the absence of other values.
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
/// disallow setting an overlapping sub-region—perhaps with an escape hatch to overwrite.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundaryOwnershipEntry {
	/// The whole of the specified boundary segment, i.e. one side of the cell's aabb.
	Whole(BoundaryOwnershipStatus),
	/// A sub-region of the specified boundary segment, i.e. a part of one side of the cell's aabb.
	///
	/// [`Aabb2d`] keys are boundary-local (see [`BoundaryRegionList`]).
	SubRegions(BoundaryRegionList<BoundaryOwnershipStatus>),
}

impl Default for BoundaryOwnershipEntry {
	fn default() -> Self {
		Self::Whole(BoundaryOwnershipStatus::Cell)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CirculationRequestStatus {
	/// The circulation request has already been fulfilled in the hierarchy.
	///
	/// This is not typically stored; it is implied by the absence of other values.
	#[default]
	Satisfied,
	/// The hierarchy has determined that some kind of circulation element is required at this boundary region.
	Required,
	/// The hierarchy has determined that some kind of circulation element is desired at this boundary region—if the child can make it work.
	Desired,
	/// The hierarchy has suggested this could be a good place for a circulation element, but it is not required or desired.
	Optional,
}

/// The description of the total circulation requirements for a region on the boundary.
///
/// Unlike ownership, circulation requirements are not assumed exclusive,
/// though they often have to be combined with the ownership table to determine the final requirements.
/// [`CellConstraints::subset`] drops circulation on regions that are not cell-owned.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CirculationEntry(pub BoundaryRegionList<Vec<CirculationRequestStatus>>);

/// A sample of the incoming boundary geometry before the joint point.
///
/// A series of three dimensional points, probably normalized to boundary space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PreJointSweep(pub Vec<Vec3>);

/// The coordinate of a joint on the boundary.
///
/// Interestingly, joints will typically fall on the edge of ownership boundaries.
/// We should probably provide a behavioral unification path for this case.
///
/// `t` / `h` are boundary-local in \([0, 1]\) (see [`BoundaryRegionList`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointCoordinate {
	/// The distance along the boundary segment.
	pub t: f32,
	/// The height up the boundary segment.
	pub h: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct JointEntry(pub Vec<(JointCoordinate, PreJointSweep)>);

/// The complete boundary table for the cell.
///
/// Often, a lower order cell will subset boundary table from its parent,
/// add its own boundary table, and then pass it down to its children.
///
/// More complicated scenarios are better handled by
/// the child simply taking a reference to parents, grandparent, etc.
/// at generation time. But, this can be a useful common parameterization.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CellBoundaryTable<T> {
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

/// Failure modes for [`CellConstraints::subset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubsetError {
	/// Child AABB is not contained in the parent cell AABB.
	NotContained,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CellConstraints {
	/// The spatial constraints of the cell; within these bounds, the cell has write authority.
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

impl CellConstraints {
	/// Minimal cell-owned constraints for `aabb` (default thickness, empty circulation/joints).
	pub fn cell_owned(aabb: Aabb3d) -> Self {
		Self {
			aabb,
			boundary_thickness: CellBoundaryTable {
				top: Some(BoundaryThicknessEntry::default()),
				bottom: Some(BoundaryThicknessEntry::default()),
				left: Some(BoundaryThicknessEntry::default()),
				right: Some(BoundaryThicknessEntry::default()),
				front: Some(BoundaryThicknessEntry::default()),
				back: Some(BoundaryThicknessEntry::default()),
			},
			boundary_ownership: CellBoundaryTable {
				top: Some(BoundaryOwnershipEntry::default()),
				bottom: Some(BoundaryOwnershipEntry::default()),
				left: Some(BoundaryOwnershipEntry::default()),
				right: Some(BoundaryOwnershipEntry::default()),
				front: Some(BoundaryOwnershipEntry::default()),
				back: Some(BoundaryOwnershipEntry::default()),
			},
			circulation: CellBoundaryTable::default(),
			joints: CellBoundaryTable::default(),
		}
	}

	/// Subset this cell into a child AABB.
	///
	/// - Child AABB must be contained in `self.aabb` (epsilon-tolerant).
	/// - Coincident faces inherit and clip boundary-local tables into the child's face space.
	/// - Interior faces become cell-owned with default thickness and empty circulation/joints.
	/// - Circulation on non-cell-owned regions is dropped.
	pub fn subset(&self, child_aabb: Aabb3d) -> Result<CellConstraints, SubsetError> {
		if !contains_aabb(&self.aabb, &child_aabb, FACE_EPS) {
			return Err(SubsetError::NotContained);
		}

		let faces = FaceKind::ALL;
		let mut ownership = CellBoundaryTable::default();
		let mut thickness = CellBoundaryTable::default();
		let mut circulation = CellBoundaryTable::default();
		let mut joints = CellBoundaryTable::default();

		for face in faces {
			let shared = face.is_coincident(&self.aabb, &child_aabb, FACE_EPS);
			if !shared {
				set_face(
					&mut ownership,
					face,
					Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Cell)),
				);
				set_face(
					&mut thickness,
					face,
					Some(BoundaryThicknessEntry::default()),
				);
				set_face(&mut circulation, face, None);
				set_face(&mut joints, face, None);
				continue;
			}

			let coverage = face.child_coverage_in_parent_local(&self.aabb, &child_aabb);
			let owned = match get_face(&self.boundary_ownership, face) {
				Some(entry) => clip_ownership(entry, coverage),
				None => BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Cell),
			};
			let thick = match get_face(&self.boundary_thickness, face) {
				Some(entry) => clip_thickness(entry, coverage),
				None => BoundaryThicknessEntry::default(),
			};
			let circ = match get_face(&self.circulation, face) {
				Some(entry) => filter_circulation(&owned, clip_circulation(entry, coverage)),
				None => CirculationEntry::default(),
			};
			let joint = match get_face(&self.joints, face) {
				Some(entry) => clip_joints(entry, coverage),
				None => JointEntry::default(),
			};

			set_face(&mut ownership, face, Some(owned));
			set_face(&mut thickness, face, Some(thick));
			set_face(
				&mut circulation,
				face,
				if circ.0.is_empty() {
					None
				} else {
					Some(circ)
				},
			);
			set_face(
				&mut joints,
				face,
				if joint.0.is_empty() {
					None
				} else {
					Some(joint)
				},
			);
		}

		Ok(CellConstraints {
			aabb: child_aabb,
			boundary_thickness: thickness,
			boundary_ownership: ownership,
			circulation,
			joints,
		})
	}
}

#[derive(Clone, Copy)]
enum FaceKind {
	Top,
	Bottom,
	Left,
	Right,
	Front,
	Back,
}

impl FaceKind {
	const ALL: [Self; 6] = [
		Self::Top,
		Self::Bottom,
		Self::Left,
		Self::Right,
		Self::Front,
		Self::Back,
	];

	fn is_coincident(self, parent: &Aabb3d, child: &Aabb3d, eps: f32) -> bool {
		match self {
			Self::Top => (child.max.y - parent.max.y).abs() <= eps,
			Self::Bottom => (child.min.y - parent.min.y).abs() <= eps,
			Self::Left => (child.min.x - parent.min.x).abs() <= eps,
			Self::Right => (child.max.x - parent.max.x).abs() <= eps,
			Self::Front => (child.min.z - parent.min.z).abs() <= eps,
			Self::Back => (child.max.z - parent.max.z).abs() <= eps,
		}
	}

	/// Child's footprint on this face, in parent boundary-local \([0,1]^2\).
	fn child_coverage_in_parent_local(self, parent: &Aabb3d, child: &Aabb3d) -> Aabb2d {
		let (t0, t1, h0, h1) = match self {
			Self::Top | Self::Bottom => {
				let dx = (parent.max.x - parent.min.x).max(FACE_EPS);
				let dz = (parent.max.z - parent.min.z).max(FACE_EPS);
				(
					(child.min.x - parent.min.x) / dx,
					(child.max.x - parent.min.x) / dx,
					(child.min.z - parent.min.z) / dz,
					(child.max.z - parent.min.z) / dz,
				)
			}
			Self::Left | Self::Right => {
				let dz = (parent.max.z - parent.min.z).max(FACE_EPS);
				let dy = (parent.max.y - parent.min.y).max(FACE_EPS);
				(
					(child.min.z - parent.min.z) / dz,
					(child.max.z - parent.min.z) / dz,
					(child.min.y - parent.min.y) / dy,
					(child.max.y - parent.min.y) / dy,
				)
			}
			Self::Front | Self::Back => {
				let dx = (parent.max.x - parent.min.x).max(FACE_EPS);
				let dy = (parent.max.y - parent.min.y).max(FACE_EPS);
				(
					(child.min.x - parent.min.x) / dx,
					(child.max.x - parent.min.x) / dx,
					(child.min.y - parent.min.y) / dy,
					(child.max.y - parent.min.y) / dy,
				)
			}
		};
		Aabb2d {
			min: Vec2::new(t0.clamp(0.0, 1.0), h0.clamp(0.0, 1.0)),
			max: Vec2::new(t1.clamp(0.0, 1.0), h1.clamp(0.0, 1.0)),
		}
	}
}

fn get_face<T>(table: &CellBoundaryTable<T>, face: FaceKind) -> Option<&T> {
	match face {
		FaceKind::Top => table.top.as_ref(),
		FaceKind::Bottom => table.bottom.as_ref(),
		FaceKind::Left => table.left.as_ref(),
		FaceKind::Right => table.right.as_ref(),
		FaceKind::Front => table.front.as_ref(),
		FaceKind::Back => table.back.as_ref(),
	}
}

fn set_face<T>(table: &mut CellBoundaryTable<T>, face: FaceKind, value: Option<T>) {
	match face {
		FaceKind::Top => table.top = value,
		FaceKind::Bottom => table.bottom = value,
		FaceKind::Left => table.left = value,
		FaceKind::Right => table.right = value,
		FaceKind::Front => table.front = value,
		FaceKind::Back => table.back = value,
	}
}

fn contains_aabb(parent: &Aabb3d, child: &Aabb3d, eps: f32) -> bool {
	child.min.x >= parent.min.x - eps
		&& child.min.y >= parent.min.y - eps
		&& child.min.z >= parent.min.z - eps
		&& child.max.x <= parent.max.x + eps
		&& child.max.y <= parent.max.y + eps
		&& child.max.z <= parent.max.z + eps
		&& child.min.x <= child.max.x
		&& child.min.y <= child.max.y
		&& child.min.z <= child.max.z
}

fn intersect_aabb2d(a: Aabb2d, b: Aabb2d) -> Option<Aabb2d> {
	let min = Vec2::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
	let max = Vec2::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
	if min.x <= max.x + FACE_EPS && min.y <= max.y + FACE_EPS {
		Some(Aabb2d { min, max })
	} else {
		None
	}
}

/// Remap a region from parent-local into child-local given the child's coverage on the parent face.
fn remap_to_child_local(region: Aabb2d, coverage: Aabb2d) -> Option<Aabb2d> {
	let clipped = intersect_aabb2d(region, coverage)?;
	let size = coverage.max - coverage.min;
	let sx = size.x.max(FACE_EPS);
	let sy = size.y.max(FACE_EPS);
	Some(Aabb2d {
		min: Vec2::new(
			((clipped.min.x - coverage.min.x) / sx).clamp(0.0, 1.0),
			((clipped.min.y - coverage.min.y) / sy).clamp(0.0, 1.0),
		),
		max: Vec2::new(
			((clipped.max.x - coverage.min.x) / sx).clamp(0.0, 1.0),
			((clipped.max.y - coverage.min.y) / sy).clamp(0.0, 1.0),
		),
	})
}

fn clip_region_list<T: Clone>(
	regions: &BoundaryRegionList<T>,
	coverage: Aabb2d,
) -> BoundaryRegionList<T> {
	regions
		.iter()
		.filter_map(|(region, value)| {
			remap_to_child_local(*region, coverage).map(|local| (local, value.clone()))
		})
		.collect()
}

fn clip_ownership(entry: &BoundaryOwnershipEntry, coverage: Aabb2d) -> BoundaryOwnershipEntry {
	match entry {
		BoundaryOwnershipEntry::Whole(status) => BoundaryOwnershipEntry::Whole(*status),
		BoundaryOwnershipEntry::SubRegions(regions) => {
			BoundaryOwnershipEntry::SubRegions(clip_region_list(regions, coverage))
		}
	}
}

fn clip_thickness(entry: &BoundaryThicknessEntry, coverage: Aabb2d) -> BoundaryThicknessEntry {
	BoundaryThicknessEntry {
		fallback: entry.fallback,
		sub_regions: clip_region_list(&entry.sub_regions, coverage),
	}
}

fn clip_circulation(entry: &CirculationEntry, coverage: Aabb2d) -> CirculationEntry {
	CirculationEntry(clip_region_list(&entry.0, coverage))
}

fn clip_joints(entry: &JointEntry, coverage: Aabb2d) -> JointEntry {
	let size = coverage.max - coverage.min;
	let sx = size.x.max(FACE_EPS);
	let sy = size.y.max(FACE_EPS);
	JointEntry(
		entry
			.0
			.iter()
			.filter_map(|(coord, sweep)| {
				if coord.t < coverage.min.x - FACE_EPS
					|| coord.t > coverage.max.x + FACE_EPS
					|| coord.h < coverage.min.y - FACE_EPS
					|| coord.h > coverage.max.y + FACE_EPS
				{
					return None;
				}
				Some((
					JointCoordinate {
						t: ((coord.t - coverage.min.x) / sx).clamp(0.0, 1.0),
						h: ((coord.h - coverage.min.y) / sy).clamp(0.0, 1.0),
					},
					sweep.clone(),
				))
			})
			.collect(),
	)
}

fn region_is_cell_owned(ownership: &BoundaryOwnershipEntry, region: Aabb2d) -> bool {
	match ownership {
		BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Cell) => true,
		BoundaryOwnershipEntry::Whole(_) => false,
		BoundaryOwnershipEntry::SubRegions(regions) => regions.iter().any(|(owned_region, status)| {
			*status == BoundaryOwnershipStatus::Cell
				&& intersect_aabb2d(*owned_region, region).is_some()
		}),
	}
}

fn filter_circulation(
	ownership: &BoundaryOwnershipEntry,
	circulation: CirculationEntry,
) -> CirculationEntry {
	CirculationEntry(
		circulation
			.0
			.into_iter()
			.filter(|(region, _)| region_is_cell_owned(ownership, *region))
			.collect(),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;

	#[test]
	fn subset_rejects_uncontained_child() -> anyhow::Result<()> {
		let parent = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(10.0, 10.0, 10.0),
		));
		let child = Aabb3d::from_min_max(Vec3::new(-1.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0));
		assert_eq!(parent.subset(child).err(), Some(SubsetError::NotContained));
		Ok(())
	}

	#[test]
	fn subset_interior_faces_are_cell_owned() -> anyhow::Result<()> {
		let parent = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(10.0, 10.0, 10.0),
		));
		let child = Aabb3d::from_min_max(Vec3::new(2.0, 2.0, 2.0), Vec3::new(4.0, 4.0, 4.0));
		let sub = parent.subset(child)?;
		assert_eq!(
			sub.boundary_ownership.left,
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Cell))
		);
		assert!(sub.circulation.left.is_none());
		Ok(())
	}

	#[test]
	fn subset_shared_bottom_inherits_and_filters_circulation() -> anyhow::Result<()> {
		let mut parent = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(10.0, 10.0, 10.0),
		));
		parent.boundary_ownership.bottom =
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Parent));
		parent.circulation.bottom = Some(CirculationEntry(vec![(
			Aabb2d {
				min: Vec2::new(0.0, 0.0),
				max: Vec2::new(1.0, 1.0),
			},
			vec![CirculationRequestStatus::Required],
		)]));

		let child = Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(5.0, 5.0, 5.0));
		let sub = parent.subset(child)?;
		assert_eq!(
			sub.boundary_ownership.bottom,
			Some(BoundaryOwnershipEntry::Whole(BoundaryOwnershipStatus::Parent))
		);
		assert!(sub.circulation.bottom.is_none());
		Ok(())
	}
}
