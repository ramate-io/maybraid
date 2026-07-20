//! Canyon family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::Canyon;

define_jersey_family! {
	layout: CanyonLowPassControllerLayout,
	bootstrap_layout: BootstrapCanyonLowPassControllerLayout / bootstrap_canyon_low_pass_controller_layout,
	controller: CanyonLowPassControllerCell,
	stamp: CanyonLowPassStampCell,
	leaves_fn: original_ids_for_canyon_low_pass_leaves,
	family_salt: 44,
	cell_size: MACRO_CELL_SIZE * 3.0,
	origin_offset: (MACRO_CELL_SIZE * 0.25, 0.0),
	likelihood: 0.78,
	occupancy_frequency: 1.0 / (MACRO_CELL_SIZE * 12.0),
	config_family: canyon,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		Canyon::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}

define_jersey_family! {
	layout: CanyonHighPassControllerLayout,
	bootstrap_layout: BootstrapCanyonHighPassControllerLayout / bootstrap_canyon_high_pass_controller_layout,
	controller: CanyonHighPassControllerCell,
	stamp: CanyonHighPassStampCell,
	leaves_fn: original_ids_for_canyon_high_pass_leaves,
	family_salt: 144,
	cell_size: MACRO_CELL_SIZE * 20.0,
	origin_offset: (MACRO_CELL_SIZE * 4.0, MACRO_CELL_SIZE * 1.0),
	likelihood: 0.24,
	occupancy_frequency: 1.0 / (MACRO_CELL_SIZE * 80.0),
	config_family: canyon,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		Canyon::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
