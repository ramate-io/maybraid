//! Painted furniture assemblies fitted into Richmond [`FurnitureNode`] slots.
//!
//! Chico-shaped: `FooParams` → `params.build()` → `Foo`. [`unit_from_num`](bed::BedParams::unit_from_num)
//! keys the palette only. Parts explode for [`material_ref::MaterialRef`] paint
//! and instance as posed cuboids until the GLBs in [`assets`] exist.
//!
//! Kit remap and unit-slot lips live in [`kit_space`]. Slot facing / abutment
//! come from the Richmond node (`placement.yaw`); assemblies do not re-read a
//! floor plan.

pub mod assets;
pub mod bed;
pub mod chair;
pub mod chest;
pub mod counter;
pub mod fill;
pub mod kit_space;
pub mod palette;
pub mod parts;
pub mod plugin;
pub mod present;

pub use bed::{Bed, BedParams};
pub use chair::{Chair, ChairParams};
pub use chest::{Chest, ChestParams};
pub use counter::{Counter, CounterParams};
pub use fill::{abutment_wall, posed_assembly, try_assembly};
pub use parts::{pose_parts, Assembly, PartKind, PlacedPart};
pub use plugin::{FurnitureAssembliesPlugin, FurnitureKitMeshes};
pub use present::{assembly_scene, filled_slot_scene, filled_slots_scene, posed_mesh_material_ref};
