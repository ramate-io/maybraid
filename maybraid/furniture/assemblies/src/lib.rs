//! Painted furniture assemblies fitted into Richmond [`FurnitureNode`](richmond_building_components::FurnitureNode) slots.
//!
//! Chico-shaped: `FooParams` → `params.build()` → `Foo`. [`unit_from_num`](bed::BedParams::unit_from_num)
//! keys the palette only. Parts explode for [`material_ref::MaterialRef`] paint
//! and instance as posed GLBs via [`furniture_components`].
//!
//! Kit remap lives in furniture-components. Slot facing / abutment come from
//! the Richmond node (`placement.yaw`); assemblies do not re-read a floor plan.

pub mod bed;
pub mod chair;
pub mod chest;
pub mod counter;
pub mod fill;
pub mod palette;
pub mod plugin;
pub mod present;

pub use bed::{Bed, BedParams};
pub use chair::{Chair, ChairParams};
pub use chest::{Chest, ChestParams};
pub use counter::{Counter, CounterParams};
pub use fill::{abutment_wall, posed_assembly, try_assembly};
pub use furniture_components::{
	assembly_scene, pose_parts, posed_kit, PartKind, PlacedPart, BOX_KIT_TO_UNIT, LEG_KIT_TO_UNIT,
};
pub use plugin::{FurnitureAssembliesPlugin, FurnitureKitMeshes};
pub use present::{filled_slot_scene, filled_slots_scene};

use richmond_building_components::FurnitureGeometry;

/// Built assembly: topology is seed-invariant; [`MaterialRef`](material_ref::MaterialRef) is not.
#[derive(Clone, Debug, PartialEq)]
pub struct Assembly {
	pub geometry: FurnitureGeometry,
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}
