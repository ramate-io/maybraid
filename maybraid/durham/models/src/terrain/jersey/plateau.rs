//! Plateau family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::PlateauCap;

define_jersey_family! {
	layout: PlateauLowPassControllerLayout,
	bootstrap_layout: BootstrapPlateauLowPassControllerLayout / bootstrap_plateau_low_pass_controller_layout,
	controller: PlateauLowPassControllerCell,
	stamp: PlateauLowPassStampCell,
	leaves_fn: original_ids_for_plateau_low_pass_leaves,
	family_salt: 22,
	cell_size: MACRO_CELL_SIZE * 6.0,
	origin_offset: (0.0, 0.0),
	config_family: plateau,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		PlateauCap::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}

define_jersey_family! {
	layout: PlateauHighPassControllerLayout,
	bootstrap_layout: BootstrapPlateauHighPassControllerLayout / bootstrap_plateau_high_pass_controller_layout,
	controller: PlateauHighPassControllerCell,
	stamp: PlateauHighPassStampCell,
	leaves_fn: original_ids_for_plateau_high_pass_leaves,
	family_salt: 122,
	cell_size: MACRO_CELL_SIZE * 40.0,
	origin_offset: (MACRO_CELL_SIZE * 1.5, MACRO_CELL_SIZE * 0.5),
	config_family: plateau,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		PlateauCap::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
