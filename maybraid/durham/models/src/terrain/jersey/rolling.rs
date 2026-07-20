//! Rolling-ground family: dual-band guillotine controller grids + leaf stamps.

use crate::terrain::cell::MACRO_CELL_SIZE;
use crate::terrain::jersey::family_macro::define_jersey_family;
use jersey_terrain_stamps::RollingGround;

define_jersey_family! {
	layout: RollingLowPassControllerLayout,
	bootstrap_layout: BootstrapRollingLowPassControllerLayout / bootstrap_rolling_low_pass_controller_layout,
	controller: RollingLowPassControllerCell,
	stamp: RollingLowPassStampCell,
	leaves_fn: original_ids_for_rolling_low_pass_leaves,
	family_salt: 66,
	cell_size: MACRO_CELL_SIZE * 1.5,
	origin_offset: (MACRO_CELL_SIZE * 0.5, 0.0),
	config_family: rolling,
	config_band: low_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RollingGround::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}

define_jersey_family! {
	layout: RollingHighPassControllerLayout,
	bootstrap_layout: BootstrapRollingHighPassControllerLayout / bootstrap_rolling_high_pass_controller_layout,
	controller: RollingHighPassControllerCell,
	stamp: RollingHighPassStampCell,
	leaves_fn: original_ids_for_rolling_high_pass_leaves,
	family_salt: 166,
	cell_size: MACRO_CELL_SIZE * 10.0,
	origin_offset: (MACRO_CELL_SIZE * 2.5, MACRO_CELL_SIZE * 1.5),
	config_family: rolling,
	config_band: high_pass,
	|bounds, seed, height_at, params| {
		let _ = height_at;
		RollingGround::from_bounds(bounds, seed, params)
			.stamp
			.modulations
	}
}
