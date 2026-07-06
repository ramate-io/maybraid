use super::test_utils::*;
use crate::gen::{BaseSpatialIndex, GeneratingSpatialIndex, Id, MaterializeStatus};
use anyhow::Result;

#[test]
fn vegetation_materializes_terrain_dependency_and_tree_dependant() -> Result<()> {
	let mut index = WorldIndex::default();
	let veg_id = Id::from_cell(cell(3.0));
	let lod = TestLod::new(cell(3.0));

	assert_eq!(
		GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, veg_id, &lod.lod_ref()),
		Some(MaterializeStatus::Created)
	);

	let terrain_id = Id::from_cell(cell(3.0));
	assert!(BaseSpatialIndex::<Terrain>::get(&index, terrain_id).is_some());
	assert!(BaseSpatialIndex::<Vegetation>::get(&index, veg_id).is_some());
	assert!(BaseSpatialIndex::<Tree>::get(&index, tree_id(veg_id)).is_some());

	assert_eq!(
		GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, veg_id, &lod.lod_ref()),
		Some(MaterializeStatus::Existing)
	);

	Ok(())
}
