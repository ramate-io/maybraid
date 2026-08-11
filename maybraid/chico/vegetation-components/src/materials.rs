//! Standard [`material_ref::MaterialRef`] identifiers for Chico vegetation.

use material_ref::MaterialRef;

/// Named recipe resolved to Chico leaf material by playground / domain libs.
pub const CHICO_LEAF_MATERIAL: &str = "CHICO_LEAF_MATERIAL";

/// Named recipe resolved to Chico stick material by playground / domain libs.
pub const CHICO_STICK_MATERIAL: &str = "CHICO_STICK_MATERIAL";

/// Canopy / foliage default: leaf shader recipe.
pub fn chico_leaf_material_ref() -> MaterialRef {
	MaterialRef::named(CHICO_LEAF_MATERIAL)
}

/// Stick / bark default: stick shader recipe.
pub fn chico_stick_material_ref() -> MaterialRef {
	MaterialRef::named(CHICO_STICK_MATERIAL)
}
