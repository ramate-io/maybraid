//! Passage-face clearance packing for usage areas (stalls, rooms, …).
//!
//! Collect long faces of [`OpeningLabel::Passage`] openings on a plan host and
//! build inward keep-out bands so furniture / regions stay clear of the entry.

use bevy_math::bounding::Aabb2d;
use procedural_common::{aabb3_to_plan, PlanAxes, PlanOpeningFace};

use crate::fit::Confines;
use crate::openings::OpeningLabel;

/// Default inward clearance kept free in front of passages (m).
pub const PASSAGE_CLEARANCE: f32 = 1.0;

/// Host / passage face helpers on the XZ plan.
pub struct PlanHost;

impl PlanHost {
	/// Four cardinal host faces (XZ plan).
	pub fn faces(host: Aabb2d) -> [PlanOpeningFace; 4] {
		[
			PlanOpeningFace {
				thru_is_x: true,
				thru: host.min.x,
				along0: host.min.y,
				along1: host.max.y,
				inward_positive: true,
			},
			PlanOpeningFace {
				thru_is_x: true,
				thru: host.max.x,
				along0: host.min.y,
				along1: host.max.y,
				inward_positive: false,
			},
			PlanOpeningFace {
				thru_is_x: false,
				thru: host.min.y,
				along0: host.min.x,
				along1: host.max.x,
				inward_positive: true,
			},
			PlanOpeningFace {
				thru_is_x: false,
				thru: host.max.y,
				along0: host.min.x,
				along1: host.max.x,
				inward_positive: false,
			},
		]
	}

	pub fn same_wall(a: PlanOpeningFace, b: PlanOpeningFace) -> bool {
		a.thru_is_x == b.thru_is_x && (a.thru - b.thru).abs() < 0.2
	}

	/// Host walls that do not carry a customer passage face.
	pub fn free_faces(host: Aabb2d, passage_faces: &[PlanOpeningFace]) -> Vec<PlanOpeningFace> {
		Self::faces(host)
			.into_iter()
			.filter(|wall| !passage_faces.iter().any(|p| Self::same_wall(*wall, *p)))
			.collect()
	}
}

/// Collect passage opening faces and build inward clearance bands.
pub struct PassageClearance;

impl PassageClearance {
	pub fn collect_faces(confines: &Confines, host: Aabb2d) -> Vec<PlanOpeningFace> {
		let mut out = Vec::new();
		for (_id, opening) in confines.openings.iter() {
			if !matches!(opening.label, OpeningLabel::Passage) {
				continue;
			}
			let passage_plan = aabb3_to_plan(&opening.bounds, PlanAxes::XZ);
			if let Some(face) = PlanOpeningFace::from_passage(host, passage_plan) {
				out.push(face);
			}
		}
		out
	}

	/// Inward bands of `depth` along each passage face.
	pub fn bands(host: Aabb2d, faces: &[PlanOpeningFace], depth: f32) -> Vec<Aabb2d> {
		let mut out = Vec::new();
		for &face in faces {
			let along = face.along_len();
			if let Some(band) = face.band(host, along, depth, 0.5) {
				out.push(band);
			}
		}
		out
	}

	/// [`Self::bands`] at [`PASSAGE_CLEARANCE`].
	pub fn bands_std(host: Aabb2d, faces: &[PlanOpeningFace]) -> Vec<Aabb2d> {
		Self::bands(host, faces, PASSAGE_CLEARANCE)
	}
}
