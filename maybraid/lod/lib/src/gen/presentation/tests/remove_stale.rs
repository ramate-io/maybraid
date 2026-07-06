use crate::gen::tests::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id, RegionPresenter};
use anyhow::{anyhow, Result};

#[test]
fn stale_ids_are_removed_after_the_handle_pass() -> Result<()> {
	let mut index = WorldIndex::default();
	let lod = TestLod::new(span(0.0, 9.0));
	let near = Id::from_cell(cell(0.0));
	let far = Id::from_cell(cell(8.0));
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, near, &lod.lod_ref());
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, far, &lod.lod_ref());

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(
		&mut presenter,
		&index,
		span(0.0, 9.0),
		&lod.lod_ref(),
	);
	assert!(presenter.vegetation.contains_key(&near));
	assert!(presenter.vegetation.contains_key(&far));

	// A narrower pass no longer wants `far`; the trailing remove_stale drops it.
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, cell(0.0), &lod.lod_ref());
	assert!(presenter.vegetation.contains_key(&near));
	assert!(!presenter.vegetation.contains_key(&far));
	match presenter.ops.last() {
		Some(PresenterOp::RemoveStale(removed)) => assert!(removed.contains(&far)),
		other => return Err(anyhow!("expected trailing RemoveStale, got {other:?}")),
	}

	Ok(())
}
