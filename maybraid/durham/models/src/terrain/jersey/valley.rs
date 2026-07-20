//! Valley-train family: independent guillotine controller grid + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::ValleyTrain;

define_jersey_family! {
	layout: ValleyControllerLayout,
	bootstrap_layout: BootstrapValleyControllerLayout / bootstrap_valley_controller_layout,
	controller: ValleyControllerCell,
	stamp: ValleyStampCell,
	leaves_fn: original_ids_for_valley_leaves,
	family_salt: 77,
	cell_size: MACRO_CELL_SIZE * 4.0,
	origin_offset: (MACRO_CELL_SIZE * 0.5, MACRO_CELL_SIZE * 0.5),
	config_field: valley,
	|bounds, seed, height_at, params| {
		ValleyTrain::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
