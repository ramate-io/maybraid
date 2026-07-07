use crate::gen::tests::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id, MaterializeStatus, SpatialIndex};
use anyhow::{anyhow, Result};

#[test]
fn index_materializes_full_descendant_chain_to_moss() -> Result<()> {
	let mut index = WorldIndex::default();
	let veg_id = Id::from_cell(cell(7.0));
	let lod = TestLod::new(cell(7.0));

	assert_eq!(
		GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, veg_id, &lod.lod_ref()),
		Some(MaterializeStatus::Created)
	);

	let tree = tree_id(veg_id);
	let leaf = leaf_id(tree);
	let moss = moss_id(leaf);

	assert!(SpatialIndex::<Terrain>::get(&index, Id::from_cell(cell(7.0))).is_some());
	assert!(SpatialIndex::<Vegetation>::get(&index, veg_id).is_some());
	assert!(SpatialIndex::<Tree>::get(&index, tree).is_some());
	assert!(SpatialIndex::<Leaf>::get(&index, leaf).is_some());
	assert!(SpatialIndex::<Moss>::get(&index, moss).is_some());

	// Versions strictly increase down the descendant chain.
	let tree_v = SpatialIndex::<Tree>::version(&index, tree)
		.ok_or_else(|| anyhow!("missing tree version"))?;
	let leaf_v = SpatialIndex::<Leaf>::version(&index, leaf)
		.ok_or_else(|| anyhow!("missing leaf version"))?;
	let moss_v = SpatialIndex::<Moss>::version(&index, moss)
		.ok_or_else(|| anyhow!("missing moss version"))?;
	assert!(tree_v < leaf_v);
	assert!(leaf_v < moss_v);

	Ok(())
}
