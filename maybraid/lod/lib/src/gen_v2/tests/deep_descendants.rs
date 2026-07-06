use super::test_utils::*;
use crate::gen_v2::{BaseSpatialIndex, GeneratingSpatialIndex, Id, MaterializeStatus, SceneLoader};
use anyhow::Result;

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

	assert!(BaseSpatialIndex::<Terrain>::get(&index, Id::from_cell(cell(7.0))).is_some());
	assert!(BaseSpatialIndex::<Vegetation>::get(&index, veg_id).is_some());
	assert!(BaseSpatialIndex::<Tree>::get(&index, tree).is_some());
	assert!(BaseSpatialIndex::<Leaf>::get(&index, leaf).is_some());
	assert!(BaseSpatialIndex::<Moss>::get(&index, moss).is_some());

	Ok(())
}

#[test]
fn loader_spawns_all_descendant_levels_through_materialize() -> Result<()> {
	let mut loader = VegetationLoader::new();
	let veg_id = Id::from_cell(cell(9.0));
	let lod = TestLod::new(cell(9.0));

	assert_eq!(
		loader.get_or_generate(veg_id, &lod.lod_ref()),
		Some(MaterializeStatus::Created)
	);

	let tree = tree_id(veg_id);
	let leaf = leaf_id(tree);
	let moss = moss_id(leaf);

	assert!(BaseSpatialIndex::<Tree>::get(loader.spatial_index(), tree).is_some());
	assert!(BaseSpatialIndex::<Leaf>::get(loader.spatial_index(), leaf).is_some());
	assert!(BaseSpatialIndex::<Moss>::get(loader.spatial_index(), moss).is_some());

	assert_eq!(loader.spawner.spawns.len(), 1);
	assert_eq!(loader.spawner.spawns[0].0, veg_id);
	assert_eq!(
		loader.descendant_spawns,
		vec![tree, leaf, moss],
		"each descendant level should be spawned through Materialize on the loader"
	);

	Ok(())
}
