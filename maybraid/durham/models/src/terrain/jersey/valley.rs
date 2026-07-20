//! Valley-train family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::ValleyTrain;

define_jersey_family! {
	layout: ValleyLowPassControllerLayout,
	bootstrap_layout: BootstrapValleyLowPassControllerLayout / bootstrap_valley_low_pass_controller_layout,
	controller: ValleyLowPassControllerCell,
	stamp: ValleyLowPassStampCell,
	leaves_fn: original_ids_for_valley_low_pass_leaves,
	family_salt: 77,
	cell_size: MACRO_CELL_SIZE * 6.0,
	origin_offset: (MACRO_CELL_SIZE * 0.5, MACRO_CELL_SIZE * 0.5),
	likelihood: 0.85,
	occupancy_frequency: 1.0 / (MACRO_CELL_SIZE * 12.0),
	config_family: valley,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		ValleyTrain::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}

define_jersey_family! {
	layout: ValleyHighPassControllerLayout,
	bootstrap_layout: BootstrapValleyHighPassControllerLayout / bootstrap_valley_high_pass_controller_layout,
	controller: ValleyHighPassControllerCell,
	stamp: ValleyHighPassStampCell,
	leaves_fn: original_ids_for_valley_high_pass_leaves,
	family_salt: 177,
	cell_size: MACRO_CELL_SIZE * 40.0,
	origin_offset: (MACRO_CELL_SIZE * 6.0, MACRO_CELL_SIZE * 4.0),
	likelihood: 0.28,
	occupancy_frequency: 1.0 / (MACRO_CELL_SIZE * 80.0),
	config_family: valley,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		ValleyTrain::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
