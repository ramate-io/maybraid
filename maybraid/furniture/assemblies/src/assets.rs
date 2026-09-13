//! Runtime paths for authored furniture GLBs (under `maybraid/assets`).
//!
//! Blender sources live in `maybraid/art/furniture/`. Export scene 0 into these
//! paths; skip `.blend1` autosaves. Until the GLBs land, assemblies instance
//! procedural cuboids that fill the remapped kit AABB in [`crate::kit_space`].

use richmond_building_components::AssetPath;

/// Bed frame kit (`X,Y \in [-1,1]\), \(Z \in [0,1]\)` in Blender).
pub const BEDFRAME_001: AssetPath = AssetPath::new("furniture/bed/bedframe/bedframe_001.glb");
/// Mattress kit (same authored box as the frame).
pub const MATTRESS_001: AssetPath = AssetPath::new("furniture/bed/mattress/mattress_001.glb");
/// Covers kit (same authored box as the mattress).
pub const COVERS_001: AssetPath = AssetPath::new("furniture/bed/covers/covers_001.glb");

/// Chest trunk.
pub const CHEST_TRUNK_001: AssetPath = AssetPath::new("furniture/chest/trunk/trunk_001.glb");
/// Chest lid (sits on the trunk).
pub const CHEST_LID_001: AssetPath = AssetPath::new("furniture/chest/lid/lid_001.glb");

/// Chair leg tube.
pub const CHAIR_LEG_001: AssetPath = AssetPath::new("furniture/chair/legs/chair_leg_001.glb");
/// Chair seat.
pub const CHAIR_SEAT_001: AssetPath = AssetPath::new("furniture/chair/seat/chair_seat_001.glb");
/// Chair back (authored \(+Y\)).
pub const CHAIR_BACK_001: AssetPath = AssetPath::new("furniture/chair/back/chair_back_001.glb");

/// Counter footer.
pub const COUNTER_FOOTER_001: AssetPath =
	AssetPath::new("furniture/counter/counter_footer/counter_footer_001.glb");
/// Counter volume.
pub const COUNTER_VOLUME_001: AssetPath =
	AssetPath::new("furniture/counter/counter_volume/counter_volume_001.glb");
/// Countertop (over-sails the volume).
pub const COUNTER_TOP_001: AssetPath =
	AssetPath::new("furniture/counter/countertop/countertop_001.glb");
