//! Unit tests for ValleyChain controller cuts and leaf discovery.

use crate::terrain::valley_chain::config::JerseyValleyChainLayerConfig;
use crate::terrain::valley_chain::controller::JerseyValleyChainControllerCell;
use crate::terrain::valley_chain::layout::JerseyValleyChainControllerLayout;
use anyhow::{bail, Result};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::Vec3;
use lod::gen::Id;

fn controller_cell(ix: i32, iz: i32) -> Aabb3d {
	let layout = JerseyValleyChainControllerLayout::default();
	layout.cell_bounds(ix, iz)
}

fn assert_tiles_parent(parent: Aabb3d, leaves: &[Aabb3d]) -> Result<()> {
	if leaves.is_empty() {
		bail!("expected at least one leaf");
	}
	let parent_area = (parent.max.x - parent.min.x) * (parent.max.z - parent.min.z);
	let leaf_area: f32 = leaves
		.iter()
		.map(|l| (l.max.x - l.min.x) * (l.max.z - l.min.z))
		.sum();
	if (leaf_area - parent_area).abs() > 1e-2 * parent_area.max(1.0) {
		bail!("area mismatch: parent={parent_area} leaves={leaf_area}");
	}
	for i in 0..leaves.len() {
		for j in (i + 1)..leaves.len() {
			let a = &leaves[i];
			let b = &leaves[j];
			let overlap_x = a.max.x > b.min.x && b.max.x > a.min.x;
			let overlap_z = a.max.z > b.min.z && b.max.z > a.min.z;
			if overlap_x && overlap_z {
				let ix0 = a.min.x.max(b.min.x);
				let ix1 = a.max.x.min(b.max.x);
				let iz0 = a.min.z.max(b.min.z);
				let iz1 = a.max.z.min(b.max.z);
				if (ix1 - ix0) > 1e-3 && (iz1 - iz0) > 1e-3 {
					bail!("leaves {i} and {j} overlap in interior");
				}
			}
		}
	}
	Ok(())
}

#[test]
fn controller_cuts_are_deterministic() -> Result<()> {
	let config = JerseyValleyChainLayerConfig::default();
	let cell = controller_cell(0, 0);
	let a = JerseyValleyChainControllerCell::from_config(cell, &config);
	let b = JerseyValleyChainControllerCell::from_config(cell, &config);
	assert_eq!(a.cuts, b.cuts);
	assert_eq!(a.leaf_aabbs(), b.leaf_aabbs());
	Ok(())
}

#[test]
fn controller_leaves_tile_parent() -> Result<()> {
	let config = JerseyValleyChainLayerConfig::default();
	let cell = controller_cell(1, -2);
	let controller = JerseyValleyChainControllerCell::from_config(cell, &config);
	let leaves = controller.leaf_aabbs();
	assert!(leaves.len() > 1, "expected multiple leaves, got {}", leaves.len());
	assert_tiles_parent(cell, &leaves)?;
	Ok(())
}

#[test]
fn leaf_ids_stable_from_bounds() -> Result<()> {
	let config = JerseyValleyChainLayerConfig::default();
	let cell = controller_cell(0, 0);
	let controller = JerseyValleyChainControllerCell::from_config(cell, &config);
	let leaves = controller.leaf_aabbs();
	let ids: Vec<Id> = leaves.iter().copied().map(Id::from_cell).collect();
	let ids_again: Vec<Id> = leaves.iter().copied().map(Id::from_cell).collect();
	assert_eq!(ids, ids_again);
	let mut unique = ids.clone();
	unique.sort();
	unique.dedup();
	assert_eq!(unique.len(), ids.len(), "leaf Ids must be unique");
	Ok(())
}

#[test]
fn region_query_selects_intersecting_leaves() -> Result<()> {
	let config = JerseyValleyChainLayerConfig::default();
	let cell = controller_cell(0, 0);
	let controller = JerseyValleyChainControllerCell::from_config(cell, &config);
	let leaves = controller.leaf_aabbs();
	let first = leaves
		.first()
		.copied()
		.ok_or_else(|| anyhow::anyhow!("no leaves"))?;
	let probe = Aabb3d::from_min_max(
		Vec3::new(
			(first.min.x + first.max.x) * 0.5 - 1.0,
			first.min.y,
			(first.min.z + first.max.z) * 0.5 - 1.0,
		),
		Vec3::new(
			(first.min.x + first.max.x) * 0.5 + 1.0,
			first.max.y,
			(first.min.z + first.max.z) * 0.5 + 1.0,
		),
	);
	let hit: Vec<_> = leaves
		.iter()
		.filter(|leaf| probe.intersects(*leaf))
		.collect();
	assert!(!hit.is_empty());
	assert!(hit.iter().any(|l| Id::from_cell(**l) == Id::from_cell(first)));
	Ok(())
}
