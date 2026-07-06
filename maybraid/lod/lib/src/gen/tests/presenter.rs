use super::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id, RegionPresenter, SpatialIndex};
use anyhow::{anyhow, Result};

#[test]
fn present_handles_new_ids_and_skips_unchanged_ones() -> Result<()> {
	let mut index = WorldIndex::default();
	let region = cell(5.0);
	let lod = TestLod::new(region);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate_region(
		&mut index,
		region,
		&lod.lod_ref(),
	);

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());

	let veg_id = Id::from_cell(cell(5.0));
	assert!(presenter.vegetation.contains_key(&veg_id));
	assert!(matches!(presenter.ops.last(), Some(PresenterOp::RemoveStale(_))));
	let handled = presenter
		.ops
		.iter()
		.filter(|op| matches!(op, PresenterOp::Handle(..)))
		.count();
	assert!(handled >= 1);

	// Nothing changed in storage: a second pass handles nothing.
	let ops_before = presenter.ops.len();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());
	let new_handles = presenter.ops[ops_before..]
		.iter()
		.filter(|op| matches!(op, PresenterOp::Handle(..)))
		.count();
	assert_eq!(new_handles, 0);

	Ok(())
}

#[test]
fn version_bump_causes_representation() -> Result<()> {
	let mut index = WorldIndex::default();
	let region = cell(6.0);
	let lod = TestLod::new(region);
	let veg_id = Id::from_cell(cell(6.0));
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, veg_id, &lod.lod_ref());

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());
	let first_presented = presenter
		.vegetation
		.get(&veg_id)
		.copied()
		.ok_or_else(|| anyhow!("vegetation not presented"))?;

	// Re-insert mutates storage and stamps a fresh version.
	let updated = Vegetation { cell: cell(6.0) };
	SpatialIndex::<Vegetation>::insert(&mut index, veg_id, updated, cell(6.0), &lod.lod_ref());

	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());
	let second_presented = presenter
		.vegetation
		.get(&veg_id)
		.copied()
		.ok_or_else(|| anyhow!("vegetation not re-presented"))?;

	assert!(first_presented < second_presented);

	Ok(())
}

#[test]
fn stale_ids_are_removed_after_the_handle_pass() -> Result<()> {
	let mut index = WorldIndex::default();
	let lod = TestLod::new(span(0.0, 9.0));
	let near = Id::from_cell(cell(0.0));
	let far = Id::from_cell(cell(8.0));
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, near, &lod.lod_ref());
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, far, &lod.lod_ref());

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, span(0.0, 9.0), &lod.lod_ref());
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

#[test]
fn present_all_chains_descendant_layers() -> Result<()> {
	let mut index = WorldIndex::default();
	let region = cell(9.0);
	let lod = TestLod::new(region);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate_region(
		&mut index,
		region,
		&lod.lod_ref(),
	);

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present_all(&mut presenter, &index, region, &lod.lod_ref());

	let veg_id = Id::from_cell(cell(9.0));
	let tree = tree_id(veg_id);
	let leaf = leaf_id(tree);
	let moss = moss_id(leaf);

	assert!(presenter.terrain.contains_key(&Id::from_cell(cell(9.0))));
	assert!(presenter.vegetation.contains_key(&veg_id));
	assert!(presenter.trees.contains_key(&tree));
	assert!(presenter.leaves.contains_key(&leaf));
	assert!(presenter.moss.contains_key(&moss));

	Ok(())
}
