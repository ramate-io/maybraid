use super::test_utils::*;
use crate::gen_v2::{BaseSpatialIndex, Id, MaterializeStatus, SceneLoader};
use anyhow::Result;

#[test]
fn get_or_generate_spawns_vegetation_and_descendant_tree() -> Result<()> {
	let mut loader = VegetationLoader::new();
	let veg_id = Id::from_cell(cell(5.0));
	let lod = TestLod::new(cell(5.0));

	assert_eq!(
		loader.get_or_generate(veg_id, &lod.lod_ref()),
		Some(MaterializeStatus::Created)
	);

	let terrain_id = Id::from_cell(cell(5.0));
	assert!(BaseSpatialIndex::<Terrain>::get(loader.spatial_index(), terrain_id).is_some());
	assert!(BaseSpatialIndex::<Tree>::get(loader.spatial_index(), tree_id(veg_id)).is_some());

	assert_eq!(loader.spawner.spawns.len(), 1);
	assert_eq!(loader.spawner.spawns[0].0, veg_id);
	let tree = tree_id(veg_id);
	assert_eq!(
		loader.descendant_spawns,
		vec![tree, leaf_id(tree), moss_id(leaf_id(tree))],
	);

	assert_eq!(
		loader.get_or_generate(veg_id, &lod.lod_ref()),
		Some(MaterializeStatus::Existing)
	);
	assert_eq!(loader.spawner.spawns.len(), 2);
	assert_eq!(loader.spawner.spawns[1].0, veg_id);

	Ok(())
}

#[test]
fn region_load_heals_before_spawning() -> Result<()> {
	let mut loader = VegetationLoader::new();
	let region = cell(2.0);
	let lod = TestLod::new(region);

	let loaded = loader.get_or_generate_region(region, &lod.lod_ref());
	assert!(!loaded.is_empty());
	assert_eq!(loader.spawner.heals.len(), 1);
	assert_eq!(loader.spawner.heals[0], region);
	assert!(!loader.descendant_spawns.is_empty());

	Ok(())
}
