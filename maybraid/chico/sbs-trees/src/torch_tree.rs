//! Shared Penmarch / Kamakura stick and canopy helpers (also stick + structural LOD for Rory).
//!
//! Both torches use [`StorybookTreeChain`](chico_sbs_geometry::StorybookTreeChain) with the
//! same selective upper/outer cheap-ball foliage policy; this module holds that emission
//! logic once. Rory reuses stick thinning and structural distance factors, with its own
//! joint-canopy candidate set.

mod canopy;
mod stick;

use chico_vegetation_components::VegetationStructuralLodProbe;

pub(crate) use canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};
pub(crate) use stick::{
	stick_nodes_banded, stick_nodes_high, stick_nodes_low, stick_nodes_medium, HIGH_STICK_BANDS,
};

/// Structural High / Medium / Low distance factors (multiples of characteristic radius).
/// Medium outer edge is +25% vs the shared vegetation default (12 → 15).
pub(crate) const TORCH_STRUCTURAL_HIGH_FACTOR: f32 = 3.0;
pub(crate) const TORCH_STRUCTURAL_MEDIUM_FACTOR: f32 = 15.0;
pub(crate) const TORCH_STRUCTURAL_LOW_FACTOR: f32 = 24.0;

/// Distance unit: max(horizontal footprint, half tree height) so tall torches don't
/// drop to Medium/Low while still filling the view.
pub(crate) fn structural_tree_radius(footprint_radius: f32, height: f32) -> f32 {
	footprint_radius.max(height * 0.5).max(1e-3)
}

pub(crate) fn structural_lod_probe(
	center: bevy::prelude::Vec3,
	footprint_radius: f32,
	height: f32,
) -> VegetationStructuralLodProbe {
	VegetationStructuralLodProbe::new(center, structural_tree_radius(footprint_radius, height))
		.with_factors(
			TORCH_STRUCTURAL_HIGH_FACTOR,
			TORCH_STRUCTURAL_MEDIUM_FACTOR,
			TORCH_STRUCTURAL_LOW_FACTOR,
		)
}
