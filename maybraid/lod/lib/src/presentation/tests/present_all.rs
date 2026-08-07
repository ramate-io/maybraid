use crate::gen::tests::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id};
use crate::presentation::RegionPresenter;
use anyhow::Result;

#[test]
fn present_all_delegates_to_present_with_descendants_on_index_type() -> Result<()> {
	let mut index = WorldIndex::default();
	let region = cell(9.0);
	let lod = TestLod::new(region);
	GeneratingSpatialIndex::<Vegetation>::get_or_generate_region(
		&mut index,
		region,
		&lod.lod_ref(),
	);

	let mut via_all = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present_all(&mut via_all, &index, region, &lod.lod_ref());

	let mut via_descendants = RecordingPresenter::default();
	RegionPresenter::<Vegetation, _>::present_with_descendants(
		&mut via_descendants,
		&index,
		region,
		&lod.lod_ref(),
	);

	assert_eq!(via_all.terrain, via_descendants.terrain);
	assert_eq!(via_all.vegetation, via_descendants.vegetation);
	assert_eq!(via_all.trees, via_descendants.trees);
	assert_eq!(via_all.leaves, via_descendants.leaves);
	assert_eq!(via_all.moss, via_descendants.moss);

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
