//! Standard [`material_ref::MaterialRef`] identifiers for Chico vegetation.

use material_ref::MaterialRef;

/// Named recipe resolved to Chico leaf material by playground / domain libs.
pub const CHICO_LEAF_MATERIAL: &str = "CHICO_LEAF_MATERIAL";

/// Named recipe resolved to Chico stick material by playground / domain libs.
pub const CHICO_STICK_MATERIAL: &str = "CHICO_STICK_MATERIAL";

/// Named recipe resolved to Chico frond material by playground / domain libs.
pub const CHICO_FROND_MATERIAL: &str = "CHICO_FROND_MATERIAL";

/// Named leaf shader recipe for higher-order canopy types (e.g. braid oak).
pub fn chico_leaf_material_ref() -> MaterialRef {
	MaterialRef::named(CHICO_LEAF_MATERIAL)
}

/// Named stick shader recipe for higher-order stick / trunk types.
pub fn chico_stick_material_ref() -> MaterialRef {
	MaterialRef::named(CHICO_STICK_MATERIAL)
}

/// Named frond shader recipe (palette + sway, no leaf cheese).
pub fn chico_frond_material_ref() -> MaterialRef {
	MaterialRef::named(CHICO_FROND_MATERIAL)
}
