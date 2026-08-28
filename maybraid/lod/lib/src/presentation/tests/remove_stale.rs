use crate::gen::tests::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id};
use crate::presentation::RegionPresenter;
use anyhow::{anyhow, Result};
use std::collections::HashSet;

#[test]
fn cull_hides_within_one_hit_and_despawns_when_budgeted() -> Result<()> {
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
	let remaining =
		RegionPresenter::<Vegetation, _>::cull(&mut presenter, &index, span(0.0, 9.0), &keep, 0);
	assert_eq!(remaining, 0);
	assert!(presenter.vegetation.contains_key(&far));
	assert!(presenter.hidden.contains(&far));

	let remaining =
		RegionPresenter::<Vegetation, _>::cull(&mut presenter, &index, span(0.0, 9.0), &keep, 1);
	assert_eq!(remaining, 0);
	assert!(presenter.vegetation.contains_key(&near));
	assert!(!presenter.vegetation.contains_key(&far));
	match presenter.ops.last() {
		Some(PresenterOp::RemoveStale(removed)) => assert!(removed.contains(&far)),
		other => return Err(anyhow!("expected trailing RemoveStale, got {other:?}")),
	}

	Ok(())
}

#[test]
fn cull_despawns_on_first_hit_when_budget_allows() -> Result<()> {
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

	let keep = HashSet::from([near]);
	RegionPresenter::<Vegetation, _>::cull(&mut presenter, &index, span(0.0, 9.0), &keep, 1);
	assert!(presenter.ops.iter().any(|op| matches!(op, PresenterOp::Hide(id) if *id == far)));
	assert!(!presenter.vegetation.contains_key(&far));
	assert!(presenter.vegetation.contains_key(&near));
	Ok(())
}

#[test]
fn cull_despawn_budget_is_one_id_per_slot() -> Result<()> {
	let mut index = WorldIndex::default();
	let lod = TestLod::new(span(0.0, 9.0));
	let near = Id::from_cell(cell(0.0));
	let mid = Id::from_cell(cell(4.0));
	let far = Id::from_cell(cell(8.0));
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, near, &lod.lod_ref());
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, mid, &lod.lod_ref());
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, far, &lod.lod_ref());

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(
		&mut presenter,
		&index,
		span(0.0, 9.0),
		&lod.lod_ref(),
	);

	let keep = HashSet::from([near]);
	let remaining =
		RegionPresenter::<Vegetation, _>::cull(&mut presenter, &index, span(0.0, 9.0), &keep, 1);
	assert_eq!(remaining, 0);
	assert!(presenter.ops.iter().any(|op| matches!(op, PresenterOp::Hide(id) if *id == mid)));
	assert!(presenter.ops.iter().any(|op| matches!(op, PresenterOp::Hide(id) if *id == far)));
	let still: Vec<Id> = [mid, far]
		.into_iter()
		.filter(|id| presenter.vegetation.contains_key(id))
		.collect();
	assert_eq!(still.len(), 1, "one leaving id stays Hidden until a later slot");
	assert!(presenter.hidden.contains(&still[0]));
	Ok(())
}

#[test]
fn cull_hides_leaving_id_when_tile_misses_grove() -> Result<()> {
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

	let keep = HashSet::from([near]);
	RegionPresenter::<Vegetation, _>::cull(&mut presenter, &index, span(0.0, 1.0), &keep, 0);
	assert!(
		presenter.hidden.contains(&far),
		"leaving id hides even when the cull AABB misses its grove"
	);
	assert!(presenter.vegetation.contains_key(&far));
	assert!(presenter.vegetation.contains_key(&near));
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
