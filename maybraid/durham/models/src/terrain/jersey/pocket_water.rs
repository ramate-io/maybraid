//! Pocket-water family: independent guillotine controller grid + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::PocketWater;

define_jersey_family! {
	layout: PocketWaterControllerLayout,
	bootstrap_layout: BootstrapPocketWaterControllerLayout / bootstrap_pocket_water_controller_layout,
	controller: PocketWaterControllerCell,
	stamp: PocketWaterStampCell,
	leaves_fn: original_ids_for_pocket_water_leaves,
	family_salt: 55,
	cell_size: MACRO_CELL_SIZE * 2.0,
	origin_offset: (0.0, MACRO_CELL_SIZE * 0.25),
	config_field: pocket_water,
	|bounds, seed, height_at, params| {
		PocketWater::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
