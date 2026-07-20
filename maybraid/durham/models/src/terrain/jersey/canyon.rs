//! Canyon family: independent guillotine controller grid + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::Canyon;

define_jersey_family! {
	layout: CanyonControllerLayout,
	bootstrap_layout: BootstrapCanyonControllerLayout / bootstrap_canyon_controller_layout,
	controller: CanyonControllerCell,
	stamp: CanyonStampCell,
	leaves_fn: original_ids_for_canyon_leaves,
	family_salt: 44,
	cell_size: MACRO_CELL_SIZE * 2.0,
	origin_offset: (MACRO_CELL_SIZE * 0.25, 0.0),
	config_field: canyon,
	|bounds, seed, height_at, params| {
		Canyon::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
