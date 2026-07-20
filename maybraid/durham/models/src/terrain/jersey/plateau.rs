//! Plateau family: independent guillotine controller grid + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::PlateauCap;

define_jersey_family! {
	layout: PlateauControllerLayout,
	bootstrap_layout: BootstrapPlateauControllerLayout / bootstrap_plateau_controller_layout,
	controller: PlateauControllerCell,
	stamp: PlateauStampCell,
	leaves_fn: original_ids_for_plateau_leaves,
	family_salt: 22,
	cell_size: MACRO_CELL_SIZE * 4.0,
	origin_offset: (0.0, 0.0),
	config_field: plateau,
	|bounds, seed, height_at, params| {
		PlateauCap::from_bounds(bounds, seed, params, height_at)
			.stamp
			.modulations
	}
}
