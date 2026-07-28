//! Circulation requirements on boundary regions.

use bevy_math::bounding::Aabb2d;

use crate::constraints::ownership::BoundaryOwnershipEntry;
use crate::constraints::region::{BoundaryRegionList, BoundaryRegionListExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CirculationRequestStatus {
	/// Already fulfilled in the hierarchy (often implied by absence).
	#[default]
	Satisfied,
	Required,
	Desired,
	Optional,
}

/// Circulation requirements for regions on one boundary face.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CirculationEntry(pub BoundaryRegionList<Vec<CirculationRequestStatus>>);

impl CirculationEntry {
	pub fn clip_to_coverage(&self, coverage: Aabb2d) -> Self {
		Self(self.0.clip_to_coverage(coverage))
	}

	/// Drop regions that are not cell-owned after subsetting.
	pub fn filter_cell_owned(self, ownership: &BoundaryOwnershipEntry) -> Self {
		Self(
			self.0
				.into_iter()
				.filter(|(region, _)| ownership.owns_region_as_cell(*region))
				.collect(),
		)
	}

	pub fn into_option(self) -> Option<Self> {
		if self.0.is_empty() {
			None
		} else {
			Some(self)
		}
	}
}
