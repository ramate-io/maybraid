//! Marazion **high-pass** band: large pocket water (leaf sides ≈800m–3km).

use crate::terrain::marazion::band_macro::define_marazion_band;

define_marazion_band! {
	layout: PrePocketHighPassLayout,
	bootstrap_layout: BootstrapPrePocketHighPassLayout / bootstrap_pre_pocket_high_pass_layout,
	pre_cell: PrePocketHighPassCell,
	pocket: PocketHighPassCell,
	pocket_waters: MarazionPocketWatersHighPass,
	pre_ids: original_ids_for_pre_pocket_high_pass_cells,
	pocket_ids: original_ids_for_pocket_high_pass_cells,
	pocket_waters_ids: original_ids_for_marazion_pocket_waters_high_pass_leaves,
	band_field: high_pass,
	band_pass: High,
	family_salt: 0x1270_0002,
	cell_size: (1200.0, 4500.0),
	pre_pocket_pitch: 32_000.0,
	// Must divide `pre_pocket_pitch` (debug_assert in PrePocket::containing).
	pocket_pitches: [8_000.0, 6_400.0, 4_000.0, 8_000.0],
	origin_offset: (640.0, 1280.0),
	likelihood: 1.0,
	spatial_correlation: 6000.0,
}

#[cfg(test)]
mod tests {
	use super::PrePocketHighPassLayout;

	#[test]
	fn pocket_pitches_divide_pre_pitch() {
		let pre = PrePocketHighPassLayout::PRE_POCKET_PITCH;
		for pitch in PrePocketHighPassLayout::POCKET_PITCHES {
			let n = pre / pitch;
			assert!((n - n.round()).abs() < 1e-3, "{pitch} does not divide {pre}");
		}
	}
}
