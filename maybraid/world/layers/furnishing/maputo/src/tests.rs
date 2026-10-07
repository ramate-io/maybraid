//! [`FurnitureSlots`] for a stored Les Halles development.

use bevy::ecs::system::RunSystemOnce;
use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{Res, World};
use building_components::FurnitureNode;
use buildings::{Confines, Fit};
use durham::Durham;
use furniture_usage_areas::expand_usages;
use lod::gen::Id;
use lod::hcsg::HcsgStorage;
use procedural_common::NoiseParams;
use richmond::{Built, BuiltDevelopment, DevelopmentHosts, LesHallesDevelopment};
use terrain_layer_model::OnTerrain;
use urbanization_developments::{MixedUseLesHallesDevelopment, PlacedBuilding};
use urbanization_layer_model::Urbanization;

use crate::cell::world_slot;
use crate::slots::FurnitureSlots;

type Ground = OnTerrain<Durham>;
type Urbanized = Urbanization<richmond::Richmond<Ground>>;

fn les_halles(yaw: f32) -> anyhow::Result<BuiltDevelopment> {
	let bounds = Aabb3d::from_min_max(Vec3::new(-18.0, 0.0, -18.0), Vec3::new(18.0, 10.0, 18.0));
	let (development, _) = MixedUseLesHallesDevelopment::fit_to_confines(
		&Confines::from_bounds(bounds),
		NoiseParams { seed: 1337, ..NoiseParams::default() },
	)?;
	Ok(BuiltDevelopment::LesHalles(Box::new(LesHallesDevelopment {
		cell: bounds,
		building: PlacedBuilding {
			center_xz: Vec2::ZERO,
			yaw,
			footprint: Vec2::splat(36.0),
			ground_height: 0.0,
			building: development,
		},
	})))
}

fn expected_slots(development: &BuiltDevelopment) -> Vec<FurnitureNode> {
	let mut out = Vec::new();
	for host in development.hosts() {
		let transform = host.transform();
		for node in host.furniture_nodes() {
			out.push(world_slot(transform, node));
		}
		for node in expand_usages(host.furniture_usage_nodes()) {
			out.push(world_slot(transform, node));
		}
	}
	out
}

#[test]
fn furniture_slots_match_les_halles_world_slots() -> anyhow::Result<()> {
	let built = les_halles(0.4)?;
	let expected = expected_slots(&built);
	anyhow::ensure!(!expected.is_empty(), "Les Halles should emit High slots");
	let bounds = Aabb3d::from_min_max(Vec3::new(-40.0, -8.0, -40.0), Vec3::new(40.0, 24.0, 40.0));
	let id = Id::from_cell(bounds);
	let elsewhere =
		Aabb3d::from_min_max(Vec3::new(800.0, -8.0, 800.0), Vec3::new(880.0, 24.0, 880.0));
	let other = les_halles(0.0)?;

	let mut world = World::new();
	world.init_resource::<HcsgStorage>();
	{
		let mut store = world.resource_mut::<HcsgStorage>();
		store.insert(id, Built::<Ground>::new(built), bounds);
		store.insert(Id::from_cell(elsewhere), Built::<Ground>::new(other), elsewhere);
	}

	let overlapping = world
		.run_system_once(move |read: Res<HcsgStorage>| {
			let version = read.entry::<Built<Ground>>(id).map(|entry| entry.version);
			let found = <Urbanized as FurnitureSlots>::slots_overlapping(&read, bounds);
			(version, found)
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let (version, found) = overlapping;
	let version = version.ok_or_else(|| anyhow::anyhow!("stored development"))?;
	anyhow::ensure!(found.len() == 1, "only the overlapping development");
	anyhow::ensure!(found[0].id == id, "development index id");
	anyhow::ensure!(found[0].version == version, "store version");
	anyhow::ensure!(found[0].slots == expected, "world-space slots match the host list");

	let missed = world
		.run_system_once(move |read: Res<HcsgStorage>| {
			<Urbanized as FurnitureSlots>::slots_overlapping(&read, elsewhere)
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	anyhow::ensure!(missed.len() == 1, "far development is its own overlap");
	anyhow::ensure!(missed[0].id == Id::from_cell(elsewhere));
	anyhow::ensure!(missed[0].slots != expected, "a different yaw is a different slot list");
	Ok(())
}
