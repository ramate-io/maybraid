//! Marazion **low-pass** band: small lakes (leaf sides ≈200–600m).

use crate::terrain::marazion::band_macro::define_marazion_band;

define_marazion_band! {
	layout: PrePocketLowPassLayout,
	bootstrap_layout: BootstrapPrePocketLowPassLayout / bootstrap_pre_pocket_low_pass_layout,
	pre_cell: PrePocketLowPassCell,
	pocket: PocketLowPassCell,
	lake: MarazionLakeLowPassCell,
	pre_ids: original_ids_for_pre_pocket_low_pass_cells,
	pocket_ids: original_ids_for_pocket_low_pass_cells,
	lake_ids: original_ids_for_marazion_lake_low_pass_leaves,
	band_field: low_pass,
	family_salt: 0x1270_0001,
	cell_size: (200.0, 600.0),
	pre_pocket_pitch: 1200.0,
	pocket_pitches: [600.0, 400.0, 300.0, 300.0],
	origin_offset: (187.0, 93.0),
	likelihood: 0.1,
	spatial_correlation: 600.0,
}
