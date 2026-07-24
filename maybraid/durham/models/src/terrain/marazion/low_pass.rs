//! Marazion **low-pass** band: small pocket water (leaf sides ≈200–600m).

use crate::terrain::marazion::band_macro::define_marazion_band;

define_marazion_band! {
	layout: PrePocketLowPassLayout,
	bootstrap_layout: BootstrapPrePocketLowPassLayout / bootstrap_pre_pocket_low_pass_layout,
	pre_cell: PrePocketLowPassCell,
	pocket: PocketLowPassCell,
	pocket_waters: MarazionPocketWatersLowPass,
	pre_ids: original_ids_for_pre_pocket_low_pass_cells,
	pocket_ids: original_ids_for_pocket_low_pass_cells,
	pocket_waters_ids: original_ids_for_marazion_pocket_waters_low_pass_leaves,
	band_field: low_pass,
	band_pass: Low,
	family_salt: 0x1270_0001,
	cell_size: (135.0, 737.0),
	pre_pocket_pitch: 10_000.0,
	pocket_pitches: [1600.0, 1400.0, 1300.0, 2300.0],
	origin_offset: (187.0, 93.0),
	likelihood: 0.24,
	spatial_correlation: 1.0,
}
