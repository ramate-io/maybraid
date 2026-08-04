//! Multi-cell apartment assembled from grouped plan cells.
//!
//! Interiors are left unfilled in v1 — each piece is a shell envelope so the
//! grouping is visible and residual `within` can accept later program fill.

use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillRegion, FillableRegions, FitError, SpaceKind};
use crate::openings::OpeningId;
use crate::shells::{RectFloor, RectFloorParams, RectFloorSlab};

/// One rectangular piece of an [`Apartment`].
#[derive(Debug, Clone, PartialEq)]
pub struct ApartmentPiece {
	pub cell_id: u32,
	pub confines: Confines,
	/// Optional shell for presentation; omitted when the cell is too small to wall.
	pub shell: Option<RectFloor>,
}

/// Apartment = ordered list of plan pieces (may be multi-rectangle).
#[derive(Debug, Clone, PartialEq)]
pub struct Apartment {
	pub group_id: u32,
	pub pieces: Vec<ApartmentPiece>,
}

impl Apartment {
	/// Build an apartment from already-grouped piece confines.
	///
	/// Each piece gets a floor shell when extents allow; interiors stay empty
	/// (`FillableRegions::within` carries each piece as [`SpaceKind::InternalSpace`]).
	pub fn from_pieces(
		group_id: u32,
		pieces: Vec<(u32, Confines)>,
	) -> Result<(Self, FillableRegions), FitError> {
		if pieces.is_empty() {
			return Err(FitError::TooSmall {
				reason: "apartment_pieces",
			});
		}
		let mut out_pieces = Vec::with_capacity(pieces.len());
		let mut within = Vec::new();
		for (cell_id, confines) in pieces {
			let shell = try_piece_shell(&confines);
			within.push(FillRegion::new(
				SpaceKind::InternalSpace,
				confines.clone(),
			));
			out_pieces.push(ApartmentPiece {
				cell_id,
				confines,
				shell,
			});
		}
		Ok((
			Self {
				group_id,
				pieces: out_pieces,
			},
			FillableRegions {
				within,
				atop: Vec::new(),
			},
		))
	}

	pub fn piece_count(&self) -> usize {
		self.pieces.len()
	}
}

fn try_piece_shell(confines: &Confines) -> Option<RectFloor> {
	let min = bevy_math::Vec3::from(confines.bounds.min);
	let max = bevy_math::Vec3::from(confines.bounds.max);
	let footprint = bevy_math::Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0));
	let height = (max.y - min.y).max(0.0);
	if footprint.x < 1.5 || footprint.y < 1.5 || height < 2.0 {
		return None;
	}
	let center_xz = bevy_math::Vec3::new(
		0.5 * (min.x + max.x),
		min.y,
		0.5 * (min.z + max.z),
	);
	Some(RectFloor::new(RectFloorParams {
		center_xz,
		footprint,
		storey_height: height,
		openings: confines.openings.clone(),
		floor: RectFloorSlab::Solid,
		ceiling: RectFloorSlab::None,
		..RectFloorParams::default()
	}))
}

/// Stable group tag for residual tooling (`i_apartment_apt_{group}`).
pub fn apartment_group_opening_id(scope: &str, group_id: u32) -> OpeningId {
	OpeningId::scoped(scope, "apt", group_id.to_string())
}

impl BuildingComponents for Apartment {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			if let Some(shell) = &piece.shell {
				out.extend(shell.panel_nodes_for_level(level));
			}
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for piece in &self.pieces {
			if let Some(shell) = &piece.shell {
				out.extend(shell.joint_nodes_for_level(level));
			}
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use crate::openings::Openings;

	#[test]
	fn from_pieces_emits_within_per_piece() {
		let a = Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 3.0, 4.0)),
			0.0,
			Openings::new(),
		);
		let b = Confines::new(
			Aabb3d::from_min_max(Vec3::new(4.0, 0.0, 0.0), Vec3::new(8.0, 3.0, 4.0)),
			0.0,
			Openings::new(),
		);
		let (apt, regions) = Apartment::from_pieces(0, vec![(1, a), (2, b)]).unwrap();
		assert_eq!(apt.piece_count(), 2);
		assert_eq!(regions.within.len(), 2);
		assert!(apt.pieces.iter().all(|p| p.shell.is_some()));
	}
}
