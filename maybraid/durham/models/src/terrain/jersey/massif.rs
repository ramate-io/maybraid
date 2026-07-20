//! Massif family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::RuggedMassif;

define_jersey_family! {
	layout: MassifLowPassControllerLayout,
	bootstrap_layout: BootstrapMassifLowPassControllerLayout / bootstrap_massif_low_pass_controller_layout,
	controller: MassifLowPassControllerCell,
	stamp: MassifLowPassStampCell,
	leaves_fn: original_ids_for_massif_low_pass_leaves,
	family_salt: 33,
	cell_size: MACRO_CELL_SIZE * 6.0,
	origin_offset: (MACRO_CELL_SIZE * 0.5, MACRO_CELL_SIZE * 0.5),
	config_family: massif,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RuggedMassif::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}

define_jersey_family! {
	layout: MassifHighPassControllerLayout,
	bootstrap_layout: BootstrapMassifHighPassControllerLayout / bootstrap_massif_high_pass_controller_layout,
	controller: MassifHighPassControllerCell,
	stamp: MassifHighPassStampCell,
	leaves_fn: original_ids_for_massif_high_pass_leaves,
	family_salt: 133,
	cell_size: MACRO_CELL_SIZE * 40.0,
	origin_offset: (MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 3.0),
	config_family: massif,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RuggedMassif::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}
