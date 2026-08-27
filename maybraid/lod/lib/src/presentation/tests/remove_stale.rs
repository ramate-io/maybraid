use crate::gen::tests::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id};
use crate::presentation::RegionPresenter;
use anyhow::{anyhow, Result};
use std::collections::HashSet;

#[test]
fn cull_hides_then_despawns_ids_outside_keep() -> Result<()> {
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

	let keep = HashSet::from([near]);
	RegionPresenter::<Vegetation, _>::cull(&mut presenter, &index, span(0.0, 9.0), &keep);
	assert!(presenter.vegetation.contains_key(&far));
	assert!(presenter.hidden.contains(&far));

	RegionPresenter::<Vegetation, _>::cull(&mut presenter, &index, span(0.0, 9.0), &keep);
	assert!(presenter.vegetation.contains_key(&near));
	assert!(!presenter.vegetation.contains_key(&far));
	match presenter.ops.last() {
		Some(PresenterOp::RemoveStale(removed)) => assert!(removed.contains(&far)),
		other => return Err(anyhow!("expected trailing RemoveStale, got {other:?}")),
	}

	Ok(())
}

#[test]
fn remove_stale_drops_unwanted_ids() -> Result<()> {
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
	RegionPresenter::<Vegetation, _>::remove_stale(&mut presenter, &HashSet::from([near]));
	assert!(presenter.vegetation.contains_key(&near));
	assert!(!presenter.vegetation.contains_key(&far));
	Ok(())
}
