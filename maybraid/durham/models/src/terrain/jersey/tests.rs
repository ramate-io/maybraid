//! Unit tests for per-family jersey guillotine grids.

use crate::terrain::jersey::configs::JerseyStampConfigs;
use crate::terrain::jersey::plateau::{PlateauControllerCell, PlateauControllerLayout};
use crate::terrain::jersey::shared::LeafAabbs;
use crate::terrain::jersey::valley::{ValleyControllerCell, ValleyControllerLayout};
use anyhow::{bail, Result};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::Vec3;
use lod::gen::Id;

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
	Ok(())
}

#[test]
fn plateau_cuts_are_deterministic() -> Result<()> {
	let configs = JerseyStampConfigs::default();
	let cell = PlateauControllerLayout::default().cell_bounds(0, 0);
	let a = PlateauControllerCell::from_family_config(cell, &configs.plateau);
	let b = PlateauControllerCell::from_family_config(cell, &configs.plateau);
	assert_eq!(a.cuts, b.cuts);
	assert_eq!(a.leaf_aabbs(), b.leaf_aabbs());
	Ok(())
}

#[test]
fn plateau_leaves_tile_parent() -> Result<()> {
	let configs = JerseyStampConfigs::default();
	let cell = PlateauControllerLayout::default().cell_bounds(1, -2);
	let controller = PlateauControllerCell::from_family_config(cell, &configs.plateau);
	let leaves = controller.leaf_aabbs();
	assert!(leaves.len() > 1, "expected multiple leaves, got {}", leaves.len());
	assert_tiles_parent(cell, &leaves)?;
	Ok(())
}

#[test]
fn family_controller_grids_are_offset_apart() -> Result<()> {
	let plateau = PlateauControllerLayout::default().cell_bounds(0, 0);
	let valley = ValleyControllerLayout::default().cell_bounds(0, 0);
	if (plateau.min.x - valley.min.x).abs() < 1e-3 && (plateau.min.z - valley.min.z).abs() < 1e-3
	{
		bail!("expected plateau and valley controller origins to differ");
	}
	Ok(())
}

#[test]
fn valley_leaf_ids_stable() -> Result<()> {
	let configs = JerseyStampConfigs::default();
	let cell = ValleyControllerLayout::default().cell_bounds(0, 0);
	let controller = ValleyControllerCell::from_family_config(cell, &configs.valley);
	let leaves = controller.leaf_aabbs();
	let ids: Vec<Id> = leaves.iter().copied().map(Id::from_cell).collect();
	let mut unique = ids.clone();
	unique.sort();
	unique.dedup();
	assert_eq!(unique.len(), ids.len(), "leaf Ids must be unique");
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
	assert!(leaves.iter().any(|leaf| probe.intersects(leaf)));
	Ok(())
}
