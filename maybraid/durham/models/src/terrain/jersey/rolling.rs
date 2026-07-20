//! Rolling-ground family: independent guillotine controller grid + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::RollingGround;

define_jersey_family! {
	layout: RollingControllerLayout,
	bootstrap_layout: BootstrapRollingControllerLayout / bootstrap_rolling_controller_layout,
	controller: RollingControllerCell,
	stamp: RollingStampCell,
	leaves_fn: original_ids_for_rolling_leaves,
	family_salt: 66,
	cell_size: MACRO_CELL_SIZE,
	origin_offset: (MACRO_CELL_SIZE * 0.5, 0.0),
	config_field: rolling,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RollingGround::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}
