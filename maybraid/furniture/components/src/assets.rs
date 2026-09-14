//! Runtime paths for furniture GLBs (under `maybraid/assets`).
//!
//! Sources are the Blender kits from [`ecb5a8a`](https://github.com/ramate-io/maybraid/commit/ecb5a8a31d2894311fa690cfbe8ab01cee385fba)
//! in `maybraid/art/furniture/` (skip `.blend1`). Export scene 0 into these
//! paths. Authored space is documented in [`crate::kit_space`].

use richmond_building_components::AssetPath;

/// Bed frame kit (`X,Y \in [-1,1]\), \(Z \in [0,1]\)` in Blender).
pub const BEDFRAME_001: AssetPath = AssetPath::new("furniture/bed/bedframe/bedframe_001.glb");
/// Mattress kit (same authored box as the frame).
pub const MATTRESS_001: AssetPath = AssetPath::new("furniture/bed/mattress/mattress_001.glb");
/// Covers kit (same authored box as the mattress — same slot transform).
pub const COVERS_001: AssetPath = AssetPath::new("furniture/bed/covers/covers_001.glb");

/// Chest trunk.
pub const CHEST_TRUNK_001: AssetPath = AssetPath::new("furniture/chest/trunk/trunk_001.glb");
/// Chest lid (sits on the trunk).
pub const CHEST_LID_001: AssetPath = AssetPath::new("furniture/chest/lid/lid_001.glb");
/// Chest latch (Blender \(Y \in [0,1]\) from the front, \(X,Z \in [-1,1]\)).
pub const CHEST_LATCH_001: AssetPath = AssetPath::new("furniture/chest/latch/latch_001.glb");

/// Chair leg tube.
pub const CHAIR_LEG_001: AssetPath = AssetPath::new("furniture/chair/legs/chair_leg_001.glb");
/// Chair seat.
pub const CHAIR_SEAT_001: AssetPath = AssetPath::new("furniture/chair/seat/chair_seat_001.glb");
/// Chair back (authored \(+Y\) → engine \(+Z\)).
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
