//! Mythical chain surface: painterly metal, traveling energy, staged damage.
//!
//! See the crate README for the A04 geometry contract (along-chain UV,
//! `Mesh::ATTRIBUTE_UV_1` rest arc length, per-link vertex-color ids, owned
//! runtime damage, and the halo skinning contract).

use bevy::prelude::*;

mod chain_material;
mod deteriorate;
mod flow;
mod halo;
mod hash;
mod material_lib;
mod params;
mod placeholder;
mod runtime;
mod string;

pub use chain_material::{
	claims_material_ref, claims_recipe, ChainMaterial, ChainMaterialKey, ChainMaterialPlugin,
	ChainMaterialUniform, MAX_CHAIN_LINKS,
};
pub use deteriorate::{
	bite_center, bite_radius, bite_threshold, chunk_threshold, global_severity, link_severity,
	link_weakness, point_removed, region_removed, severity, LinkLook, BITES_PER_LINK,
	CHUNKS_PER_LINK,
};
pub use flow::{chain_flow, hit_flash, ChainFlowStyle};
pub use halo::{halo_expansion_metres, ChainHalo, ChainHaloMesh, HaloMaterialIds, HALO_EXPANSION};
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
	DEFAULT_CONDUIT_FREQUENCY, DEFAULT_DAMAGE, DEFAULT_EMBER, DEFAULT_GLOW, DEFAULT_GLOW_INTENSITY,
	DEFAULT_LINK_COUNT, DEFAULT_METAL, DEFAULT_NOISE_FREQUENCY, DEFAULT_SEED,
	MYTHICAL_CHAIN_SCALAR_COUNT, PALETTE_DAMAGE, PALETTE_EMBER, PALETTE_GLOW, PALETTE_METAL,
	RECIPE_MYTHICAL_CHAIN, SCALAR_CONDUIT_FREQUENCY, SCALAR_DAMAGE, SCALAR_FADE,
	SCALAR_FLOW_BASELINE, SCALAR_FLOW_HALO_GAIN, SCALAR_FLOW_HALO_WIDTH, SCALAR_FLOW_PEAK,
	SCALAR_FLOW_SPEED, SCALAR_FLOW_WAVELENGTH, SCALAR_GLOW_INTENSITY, SCALAR_INTACT,
	SCALAR_LINK_COUNT, SCALAR_LOCALIZED_BLEND, SCALAR_RESERVED,
};
pub use placeholder::{apply_localized_damage, placeholder_chain_mesh, PlaceholderChainSpec};
pub use runtime::{
	apply_chain_hit, copy_authored_onto, owned_chain_material, ChainRuntimeDamage,
	OwnedChainHandle, OwnedChainMaterial,
};
pub use string::{
	bent_centerline, continuous_tube_mesh, rest_centerline, skinned_tube_mesh, spawn_skinned_tube,
	ContinuousTubeSpec, SKINNED_TUBE_JOINTS,
};

/// Domain shader plugin used by [`ChainMaterialLib`].
pub struct ChainShadersPlugin;

impl Plugin for ChainShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(ChainMaterialPlugin);
	}
}
