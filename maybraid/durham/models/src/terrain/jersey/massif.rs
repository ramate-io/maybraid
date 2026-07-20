//! Massif family: independent guillotine controller grid + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::RuggedMassif;

define_jersey_family! {
	layout: MassifControllerLayout,
	bootstrap_layout: BootstrapMassifControllerLayout / bootstrap_massif_controller_layout,
	controller: MassifControllerCell,
	stamp: MassifStampCell,
	leaves_fn: original_ids_for_massif_leaves,
	family_salt: 33,
	cell_size: MACRO_CELL_SIZE * 4.0,
	origin_offset: (MACRO_CELL_SIZE * 0.5, MACRO_CELL_SIZE * 0.5),
	config_field: massif,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RuggedMassif::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}
