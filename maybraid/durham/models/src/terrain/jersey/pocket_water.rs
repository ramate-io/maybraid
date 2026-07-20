//! Pocket-water family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::{MACRO_CELL_SIZE, TERRAIN_CELL_SIZE};
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::PocketWater;

define_jersey_family! {
	layout: PocketWaterLowPassControllerLayout,
	bootstrap_layout: BootstrapPocketWaterLowPassControllerLayout / bootstrap_pocket_water_low_pass_controller_layout,
	controller: PocketWaterLowPassControllerCell,
	stamp: PocketWaterLowPassStampCell,
	leaves_fn: original_ids_for_pocket_water_low_pass_leaves,
	family_salt: 55,
	cell_size: (TERRAIN_CELL_SIZE * 1.25, MACRO_CELL_SIZE * 1.5),
	controller_cell_size: MACRO_CELL_SIZE * 3.0,
	origin_offset: (0.0, MACRO_CELL_SIZE * 0.25),
	likelihood: 0.88,
	spatial_correlation: MACRO_CELL_SIZE * 12.0,
	config_family: pocket_water,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		PocketWater::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}

define_jersey_family! {
	layout: PocketWaterHighPassControllerLayout,
	bootstrap_layout: BootstrapPocketWaterHighPassControllerLayout / bootstrap_pocket_water_high_pass_controller_layout,
	controller: PocketWaterHighPassControllerCell,
	stamp: PocketWaterHighPassStampCell,
	leaves_fn: original_ids_for_pocket_water_high_pass_leaves,
	family_salt: 155,
	cell_size: (MACRO_CELL_SIZE * 2.0, MACRO_CELL_SIZE * 8.0),
	controller_cell_size: MACRO_CELL_SIZE * 20.0,
	origin_offset: (MACRO_CELL_SIZE * 3.0, MACRO_CELL_SIZE * 5.0),
	likelihood: 0.2,
	spatial_correlation: MACRO_CELL_SIZE * 80.0,
	config_family: pocket_water,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		PocketWater::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
