use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use lod::gen::SpatialIndex;
use procedural_common::NoiseParams;
use urbanization_cells::UrbanizationKind;

use crate::generation::{GroupKind, MobGroup, MobPlantHost, MobWorldSample};
use crate::index::{urban_leaf_arrival_radius, MobCell, MobCellExtent, MobIndex};

#[test]
fn mob_cells_cover_a_four_hundred_metre_lattice() {
	let region = Aabb3d::from_min_max(Vec3::new(-199.0, 0.0, -199.0), Vec3::new(201.0, 1.0, 201.0));
	assert_eq!(MobCellExtent::cells_overlapping(region).len(), 4);
}

#[test]
fn bounds_extent_keeps_the_given_rectangle() -> anyhow::Result<()> {
	let min = Vec3::new(-36.0, 0.0, -28.0);
	let max = Vec3::new(36.0, 1.0, 28.0);
	let extent = MobCellExtent::from_bounds(min, max);
	anyhow::ensure!(extent.aabb() == Aabb3d::from_min_max(min, max));
	anyhow::ensure!(MobCellExtent::from_id(extent.id()).is_none(), "grid from_id stays 400 m");
	Ok(())
}

#[test]
fn frontier_hosts_keep_urban_families_inside_the_arrival_disk() {
	let index = MobIndex::ready_frontier(vec![MobPlantHost {
		xz: Vec2::new(20.0, -8.0),
		arrival_radius: 6.0,
	}]);
	let group = MobGroup::generate(GroupKind::Frontier, 11, Vec2::ZERO, &index);
	let planted: Vec<_> = group
		.mobs
		.iter()
		.filter(|mob| {
			matches!(
				mob.scene.mob.kind,
				mob_scenes::MobKind::Guard
					| mob_scenes::MobKind::Brawler
					| mob_scenes::MobKind::Pleb
			)
		})
		.collect();
	assert!(!planted.is_empty());
	for mob in planted {
		let xz = Vec2::new(mob.transform.translation.x, mob.transform.translation.z);
		assert!(xz.distance(Vec2::new(20.0, -8.0)) <= 6.0 + 1e-4);
		assert_eq!(mob.transform.translation.y, 0.0);
	}
}

#[test]
fn urban_leaf_arrival_matches_setting_formula() {
	let bounds = Aabb3d::from_min_max(Vec3::new(-40.0, 0.0, -20.0), Vec3::new(40.0, 1.0, 20.0));
	assert_eq!(urban_leaf_arrival_radius(bounds), 10.0);
}

#[test]
fn stub_sampling_drives_the_index() -> anyhow::Result<()> {
	let mut index = MobIndex::default();
	index.configure_from(
		NoiseParams { frequency: 0.25, ..Default::default() },
		None,
		NoiseParams::default(),
		Some(UrbanizationKind::Frontier),
		stub_layers_at,
		stub_kind_at,
	);
	let sample = index.sample_mobs(Vec2::ZERO);
	anyhow::ensure!((sample.vegetation - 1.0).abs() < 1e-5, "stub layers fill every slot");
	anyhow::ensure!((sample.urbanization - 0.4).abs() < 1e-5, "frontier weight");
	Ok(())
}

fn stub_layers_at(_noise: NoiseParams, _layering: Option<chico::LayeringKind>, _xz: Vec2) -> u8 {
	4
}

fn stub_kind_at(
	_noise: NoiseParams,
	pinned: Option<UrbanizationKind>,
	_xz: Vec2,
) -> UrbanizationKind {
	pinned.unwrap_or(UrbanizationKind::None)
}

#[test]
fn insert_and_remove_one_cell() -> anyhow::Result<()> {
	let mut index = MobIndex::ready();
	let extent = MobCellExtent::from_cell_index(0, 0);
	let id = index.insert_cell(MobCell { extent, groups: Vec::new() });
	anyhow::ensure!(index.get(id).is_some(), "insert stores the cell");
	anyhow::ensure!(index.remove_cell(id).is_some(), "remove returns the cell");
	anyhow::ensure!(index.is_empty(), "remove drops the cell");
	Ok(())
}
