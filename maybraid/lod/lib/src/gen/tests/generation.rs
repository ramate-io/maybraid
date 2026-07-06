use super::test_utils::*;
use crate::gen::{GeneratingSpatialIndex, Id, MaterializeStatus, SpatialIndex};
use anyhow::{anyhow, Result};

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
	assert!(SpatialIndex::<Terrain>::get(&index, terrain_id).is_some());
	assert!(SpatialIndex::<Vegetation>::get(&index, veg_id).is_some());
	assert!(SpatialIndex::<Tree>::get(&index, tree_id(veg_id)).is_some());

	// Dependency stamps before the dependant, descendant after.
	let terrain_v = SpatialIndex::<Terrain>::version(&index, terrain_id)
		.ok_or_else(|| anyhow!("missing terrain version"))?;
	let veg_v = SpatialIndex::<Vegetation>::version(&index, veg_id)
		.ok_or_else(|| anyhow!("missing vegetation version"))?;
	let tree_v = SpatialIndex::<Tree>::version(&index, tree_id(veg_id))
		.ok_or_else(|| anyhow!("missing tree version"))?;
	assert!(terrain_v < veg_v);
	assert!(veg_v < tree_v);

	Ok(())
}

#[test]
fn regenerating_existing_id_reports_existing_and_keeps_version() -> Result<()> {
	let mut index = WorldIndex::default();
	let veg_id = Id::from_cell(cell(4.0));
	let lod = TestLod::new(cell(4.0));

	GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, veg_id, &lod.lod_ref());
	let first = SpatialIndex::<Vegetation>::version(&index, veg_id)
		.ok_or_else(|| anyhow!("missing vegetation version"))?;

	assert_eq!(
		GeneratingSpatialIndex::<Vegetation>::get_or_generate(&mut index, veg_id, &lod.lod_ref()),
		Some(MaterializeStatus::Existing)
	);
	let second = SpatialIndex::<Vegetation>::version(&index, veg_id)
		.ok_or_else(|| anyhow!("missing vegetation version"))?;

	assert_eq!(first, second);

	Ok(())
}

#[test]
fn region_generation_builds_fresh_origin_ids() -> Result<()> {
	let mut index = WorldIndex::default();
	let region = cell(2.0);
	let lod = TestLod::new(region);

	let loaded = GeneratingSpatialIndex::<Vegetation>::get_or_generate_region(
		&mut index,
		region,
		&lod.lod_ref(),
	);

	assert!(!loaded.is_empty());
	assert!(loaded.iter().any(|(id, _)| *id == Id::from_cell(cell(2.0))));
	assert!(SpatialIndex::<Vegetation>::get(&index, Id::from_cell(cell(2.0))).is_some());

	Ok(())
}
