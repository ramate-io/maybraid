//! Mythical chain surface: glow, durability, and fade on UV-locked links.
//!
//! See the crate README for the A04 geometry contract (along-chain UV,
//! per-link vertex-color ids, owned runtime damage, and the fracture atlas).

use bevy::prelude::*;

mod chain_material;
mod cracks;
mod deteriorate;
mod halo;
mod hash;
mod material_lib;
mod params;
mod placeholder;
mod pulse;
mod runtime;

pub use chain_material::{
	claims_material_ref, claims_recipe, ChainMaterial, ChainMaterialKey, ChainMaterialPlugin,
	ChainMaterialUniform, MAX_CHAIN_LINKS,
};
pub use cracks::{
	atlas_image, bake_atlas_rgba, baked_atlas_rgba, sample_atlas, write_placeholder_png,
	ChainCrackAtlas, ATLAS_SIZE, CRACK_ASPECT, VARIANT_COUNT, VARIANT_SIZE,
};
pub use deteriorate::{link_damage, link_weakness, LinkLook};
pub use halo::{halo_expansion_metres, ChainHalo, ChainHaloMesh, HALO_EXPANSION};
pub use hash::{
	hash13, hash21, HASH13_OFFSET, HASH13_SCALE, HASH21_Z, WEAKNESS_ID_SCALE, WEAKNESS_SEED_SCALE,
	WEAKNESS_Z,
};
pub use material_lib::{
	init_chain_material_caches, ChainMaterialLib, ChainMaterialRefCache, ChainMaterialRefPlugin,
	ChainStandaloneMaterialLib,
};
pub use params::{
	is_mythical_chain_recipe, mythical_chain_noise, mythical_chain_palette, mythical_chain_ref,
	mythical_chain_ref_for, mythical_chain_ref_with, MythicalChainScalars, MythicalChainState,
	DEFAULT_CRACK_FREQUENCY, DEFAULT_DAMAGE, DEFAULT_EMBER, DEFAULT_GLOW, DEFAULT_GLOW_INTENSITY,
	DEFAULT_LINK_COUNT, DEFAULT_METAL, DEFAULT_NOISE_FREQUENCY, DEFAULT_PULSE_RATE, DEFAULT_SEED,
	PALETTE_DAMAGE, PALETTE_EMBER, PALETTE_GLOW, PALETTE_METAL, RECIPE_MYTHICAL_CHAIN,
	SCALAR_CRACK_FREQUENCY, SCALAR_DAMAGE, SCALAR_FADE, SCALAR_GLOW_INTENSITY, SCALAR_INTACT,
	SCALAR_LINK_COUNT, SCALAR_LOCALIZED_BLEND, SCALAR_PULSE_RATE,
};
pub use placeholder::{apply_localized_damage, placeholder_chain_mesh, PlaceholderChainSpec};
pub use pulse::{
	chain_pulse, PULSE_CREST, PULSE_CREST_CENTER, PULSE_CREST_HALF, PULSE_LINK_VARIATION,
	PULSE_TROUGH,
};
pub use runtime::{
	apply_chain_hit, copy_authored_onto, owned_chain_material, ChainRuntimeDamage,
	OwnedChainHandle, OwnedChainMaterial,
};

/// Domain shader plugin used by [`ChainMaterialLib`].
pub struct ChainShadersPlugin;

impl Plugin for ChainShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ChainMaterialPlugin);
	}
}
