//! Grown forest tiles at playground origin: canopy, understory, and tufts.
//!
//! This is the same stacked kinds as the world-route neighborhood around
//! `(-4950, 3150)` (Temperate Lower Massives, LowBush, TallGrass), translated
//! to the playground so cameras can stand outside the canopies.

use crate::kit;
use bevy::prelude::*;
use chico::{
	ChicoGroveHost, ForestGroveKind, ForestGroveRecipe, ForestLayer, GpuFieldKind, GroveGpuVisuals,
};
use vegetation_groves::{FlatTerrainSample, GroveExtent};

/// Marker on GPU field patches and kit previews spawned for the world forest.
#[derive(Component)]
pub struct WorldForest;

const TILE_M: f32 = 100.0;

/// 2×2 canopy plus an east understory/tuft strip, all outside the default close tree.
pub fn spawn_world_forest(world: &mut World, gpu: bool) {
	clear_world_forest(world);
	hide_default_cover(world, true);
	ensure_forest_ground(world);
	GroveGpuVisuals::admit(gpu);
	for host in grown_hosts() {
		if gpu {
			spawn_gpu_host(world, &host);
		} else {
			spawn_legacy_host(world, &host);
		}
	}
	GroveGpuVisuals::admit(false);
}

pub fn clear_world_forest(world: &mut World) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<WorldForest>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
	hide_default_cover(world, false);
	GroveGpuVisuals::admit(false);
}

fn grown_hosts() -> Vec<ChicoGroveHost> {
	let mut hosts = Vec::new();
	for x in 0..=1 {
		for z in 0..=1 {
			hosts.push(grow_host(
				ForestGroveKind::TemperateLowerMassives,
				ForestLayer::LowerCanopy,
				Vec3::new(x as f32 * TILE_M, 0.0, z as f32 * TILE_M),
			));
		}
	}
	hosts.push(grow_host(
		ForestGroveKind::LowBush,
		ForestLayer::Understory,
		Vec3::new(2.0 * TILE_M, 0.0, 0.0),
	));
	hosts.push(grow_host(
		ForestGroveKind::TallGrass,
		ForestLayer::Tufts,
		Vec3::new(2.0 * TILE_M, 0.0, TILE_M),
	));
	hosts
}

fn grow_host(kind: ForestGroveKind, layer: ForestLayer, origin: Vec3) -> ChicoGroveHost {
	let extent = GroveExtent::new(origin, origin + Vec3::new(TILE_M, 1.0, TILE_M));
	let tile = ForestGroveRecipe::uniform(kind, extent).grow(&FlatTerrainSample::default());
	ChicoGroveHost::new(tile, layer)
}

fn spawn_gpu_host(world: &mut World, host: &ChicoGroveHost) {
	let visuals = host.gpu_visuals();
	if let Some(grove) = visuals.trees.clone() {
		world.spawn((
			WorldForest,
			Name::new("world-forest-trees"),
			Transform::IDENTITY,
			Visibility::default(),
			grove,
		));
	}
	if let Some(patch) = visuals.bushes.clone() {
		world.spawn((WorldForest, Name::new("world-forest-bushes"), patch));
	}
	if let Some(patch) = visuals.grass.clone() {
		world.spawn((WorldForest, Name::new("world-forest-grass"), patch));
	}
}

fn spawn_legacy_host(world: &mut World, host: &ChicoGroveHost) {
	let sites = host.tile.plant_sites();
	if sites.is_empty() {
		return;
	}
	match GroveGpuVisuals::field_kind(&host.tile, host.layer) {
		GpuFieldKind::Tree => kit::spawn_kit_from_height_sites(world, &sites),
		GpuFieldKind::Bush => kit::spawn_kit_bushes(world, &sites),
		GpuFieldKind::Grass => kit::spawn_kit_grass(world, &sites),
	}
}

fn hide_default_cover(world: &mut World, hide: bool) {
	let mut names = world.query::<(Entity, &Name)>();
	let ids: Vec<_> = names
		.iter(world)
		.filter(|(_, name)| name.as_str() == "gpu-grass-cover")
		.map(|(entity, _)| entity)
		.collect();
	drop(names);
	for id in ids {
		if let Some(mut visibility) = world.entity_mut(id).get_mut::<Visibility>() {
			*visibility = if hide { Visibility::Hidden } else { Visibility::Inherited };
		} else if hide {
			world.entity_mut(id).insert(Visibility::Hidden);
		}
	}
	let mut planes = world.query_filtered::<&mut Visibility, With<crate::ground::TurfPlane>>();
	for mut visibility in planes.iter_mut(world) {
		*visibility = if hide { Visibility::Hidden } else { Visibility::Inherited };
	}
}

fn ensure_forest_ground(world: &mut World) {
	let existing = world.query_filtered::<Entity, With<ForestGround>>().iter(world).next();
	if existing.is_some() {
		return;
	}
	let mesh = {
		let mut meshes = world.resource_mut::<Assets<Mesh>>();
		meshes.add(Plane3d::default().mesh().size(420.0, 420.0))
	};
	let material = {
		let mut materials = world.resource_mut::<Assets<StandardMaterial>>();
		materials.add(StandardMaterial {
			base_color: Color::srgb(0.42, 0.52, 0.32),
			perceptual_roughness: 0.95,
			..default()
		})
	};
	world.spawn((
		ForestGround,
		WorldForest,
		Name::new("world-forest-ground"),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		Transform::from_xyz(140.0, -0.05, 100.0),
		bevy::light::NotShadowCaster,
	));
}

#[derive(Component)]
struct ForestGround;

/// Cameras sit outside the 2×2 canopy: southwest vista, south edge, east floor.
pub const VISTA: (Vec3, Vec3) = (Vec3::new(-48.0, 26.0, -56.0), Vec3::new(90.0, 10.0, 90.0));
pub const EDGE: (Vec3, Vec3) = (Vec3::new(50.0, 12.0, 228.0), Vec3::new(70.0, 8.0, 90.0));
pub const FLOOR: (Vec3, Vec3) = (Vec3::new(318.0, 3.6, 80.0), Vec3::new(230.0, 1.4, 90.0));

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cameras_sit_outside_the_canopy_footprint() {
		assert!(VISTA.0.x < 0.0 && VISTA.0.z < 0.0);
		assert!(EDGE.0.z > 200.0);
		assert!(FLOOR.0.x > 300.0);
	}
}
