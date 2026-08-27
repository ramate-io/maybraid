use crate::gen::tests::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id, SpatialIndex};
use crate::presentation::RegionPresenter;
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
	assert!(presenter.ops.iter().all(|op| !matches!(op, PresenterOp::RemoveStale(_))));
	let handled = presenter.ops.iter().filter(|op| matches!(op, PresenterOp::Handle(..))).count();
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
fn needs_repair_rehandles_without_version_bump() -> Result<()> {
	let mut index = WorldIndex::default();
	let region = cell(6.5);
	let lod = TestLod::new(region);
	let veg_id = Id::from_cell(cell(6.5));
	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, veg_id, &lod.lod_ref());

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());
	let version = presenter
		.vegetation
		.get(&veg_id)
		.copied()
		.ok_or_else(|| anyhow!("vegetation not presented"))?;
	let handles_after_first = presenter
		.ops
		.iter()
		.filter(|op| matches!(op, PresenterOp::Handle(id, _) if *id == veg_id))
		.count();
	assert_eq!(handles_after_first, 1);

	// Storage unchanged; runtime state is wrong (entity missing, wrong parent, etc.).
	presenter.repair_ids.insert(veg_id);

	let ops_before = presenter.ops.len();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());
	let new_handles = presenter.ops[ops_before..]
		.iter()
		.filter(|op| matches!(op, PresenterOp::Handle(veg_id, v) if *v == version))
		.count();
	assert_eq!(new_handles, 1, "needs_repair should re-handle at the same version");

	Ok(())
}

#[test]
fn present_always_heals_after_handling() -> Result<()> {
	let mut index = WorldIndex::default();
	let region = cell(5.5);
	let lod = TestLod::new(region);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate_region(
		&mut index,
		region,
		&lod.lod_ref(),
	);

	let mut presenter = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());
	assert!(presenter.ops.iter().all(|op| !matches!(op, PresenterOp::RemoveStale(_))));

	// Unchanged storage: no new handles, and refresh does not cull.
	let ops_before = presenter.ops.len();
	RegionPresenter::<Vegetation, _>::present(&mut presenter, &index, region, &lod.lod_ref());
	assert_eq!(
		presenter.ops[ops_before..]
			.iter()
			.filter(|op| matches!(op, PresenterOp::Handle(..)))
			.count(),
		0
	);
	assert!(presenter.ops[ops_before..]
		.iter()
		.all(|op| !matches!(op, PresenterOp::RemoveStale(_))));

	Ok(())
}
