//! Exploded painted parts in unit-slot space.

use material_ref::MaterialRef;
use richmond_building_components::{AssetPath, FurnitureGeometry, Placement};

use crate::assets;

/// One kit piece: unit-slot placement + deferred paint.
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedPart {
	pub kind: PartKind,
	pub placement: Placement,
	pub material: MaterialRef,
}

/// Built assembly: topology is seed-invariant; [`MaterialRef`] is not.
#[derive(Clone, Debug, PartialEq)]
pub struct Assembly {
	pub geometry: FurnitureGeometry,
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

/// Authored furniture kit piece (matches `maybraid/art/furniture/`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PartKind {
	BedFrame,
	Mattress,
	Covers,
	ChestTrunk,
	ChestLid,
	ChairLeg,
	ChairSeat,
	ChairBack,
	CounterFooter,
	CounterVolume,
	CounterTop,
}

impl PartKind {
	/// Destination GLB under `maybraid/assets` once the Blender kits are exported.
	pub const fn asset_path(self) -> AssetPath {
		match self {
			Self::BedFrame => assets::BEDFRAME_001,
			Self::Mattress => assets::MATTRESS_001,
			Self::Covers => assets::COVERS_001,
			Self::ChestTrunk => assets::CHEST_TRUNK_001,
			Self::ChestLid => assets::CHEST_LID_001,
			Self::ChairLeg => assets::CHAIR_LEG_001,
			Self::ChairSeat => assets::CHAIR_SEAT_001,
			Self::ChairBack => assets::CHAIR_BACK_001,
			Self::CounterFooter => assets::COUNTER_FOOTER_001,
			Self::CounterVolume => assets::COUNTER_VOLUME_001,
			Self::CounterTop => assets::COUNTER_TOP_001,
		}
	}
}

/// Compose unit-slot parts under a stamped [`richmond_building_components::FurnitureNode`].
pub fn pose_parts(
	slot: &richmond_building_components::FurnitureNode,
	parts: &[PlacedPart],
) -> Vec<PlacedPart> {
	parts
		.iter()
		.map(|part| PlacedPart {
			kind: part.kind,
			placement: slot.placement.compose_child(part.placement),
			material: part.material.clone(),
		})
		.collect()
}
