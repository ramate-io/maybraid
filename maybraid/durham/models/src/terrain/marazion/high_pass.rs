//! Marazion **high-pass** band: large lakes (leaf sides ≈800m–3km).

use crate::terrain::marazion::band_macro::define_marazion_band;

define_marazion_band! {
	layout: PrePocketHighPassLayout,
	bootstrap_layout: BootstrapPrePocketHighPassLayout / bootstrap_pre_pocket_high_pass_layout,
	pre_cell: PrePocketHighPassCell,
	pocket: PocketHighPassCell,
	lake: MarazionLakeHighPassCell,
	pre_ids: original_ids_for_pre_pocket_high_pass_cells,
	pocket_ids: original_ids_for_pocket_high_pass_cells,
	lake_ids: original_ids_for_marazion_lake_high_pass_leaves,
	band_field: high_pass,
	family_salt: 0x1270_0002,
	cell_size: (800.0, 3000.0),
	pre_pocket_pitch: 3000.0,
	pocket_pitches: [3000.0, 1500.0, 1000.0, 1000.0],
	origin_offset: (640.0, 1280.0),
	likelihood: 0.14,
	spatial_correlation: 6000.0,
}
