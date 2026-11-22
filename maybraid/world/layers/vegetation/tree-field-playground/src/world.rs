//! Grown forest tiles at playground origin: canopy, understory, and tufts.
//!
//! This is the same stacked kinds as the world-route neighborhood around
//! `(-4950, 3150)` (Temperate Lower Massives, LowBush, TallGrass), translated
//! to the playground so cameras can stand outside the canopies.

use crate::kit;
use bevy::prelude::*;
use chico::{
	ChicoGroveHost, ForestExtent, ForestGroveKind, ForestGroveRecipe, ForestLayer, ForestSelection,
	GpuFieldKind, GroveGpuVisuals, LayeringKind, SelectedLayers, select_layering,
};
use vegetation_groves::{FlatTerrainSample, GroveExtent};

/// Marker on GPU field patches and kit previews spawned for the world forest.
#[derive(Component)]
pub struct WorldForest;

const TILE_M: f32 = 100.0;

/// 2×2 canopy plus an east understory/tuft strip, all outside the default close tree.
pub fn spawn_world_forest(world: &mut World, gpu: bool) {
	spawn_hosts(world, gpu, grown_hosts);
}

/// Hopscotch-selected stacked tiles from the world walk, translated to a playground strip.
pub fn spawn_world_walk(world: &mut World, gpu: bool) {
	spawn_hosts(world, gpu, walk_hosts);
}

/// Canopy / understory / tufts at 0, 200, 450 and 900 m so a far camera can
/// show GPU coverage through the 1 km present ring.
pub fn spawn_world_far(world: &mut World, gpu: bool) {
	spawn_hosts(world, gpu, far_hosts);
	replace_forest_ground(world, Vec2::new(520.0, 1400.0), Vec3::new(100.0, -0.05, 450.0));
}

fn spawn_hosts(world: &mut World, gpu: bool, hosts: fn() -> Vec<ChicoGroveHost>) {
	clear_world_forest(world);
	hide_default_cover(world, true);
	ensure_forest_ground(world);
	GroveGpuVisuals::admit(gpu);
	for host in hosts() {
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

fn far_hosts() -> Vec<ChicoGroveHost> {
	let mut hosts = Vec::new();
	for z in [0.0, 200.0, 450.0, 900.0] {
		hosts.push(grow_host(
			ForestGroveKind::TemperateLowerMassives,
			ForestLayer::LowerCanopy,
			Vec3::new(0.0, 0.0, z),
		));
		hosts.push(grow_host(
			ForestGroveKind::LowBush,
			ForestLayer::Understory,
			Vec3::new(100.0, 0.0, z),
		));
		hosts.push(grow_host(
			ForestGroveKind::TallGrass,
			ForestLayer::Tufts,
			Vec3::new(200.0, 0.0, z),
		));
	}
	hosts
}

fn grow_host(kind: ForestGroveKind, layer: ForestLayer, origin: Vec3) -> ChicoGroveHost {
	let extent = GroveExtent::new(origin, origin + Vec3::new(TILE_M, 1.0, TILE_M));
	let tile = ForestGroveRecipe::uniform(kind, extent).grow(&FlatTerrainSample::default());
	ChicoGroveHost::new(tile, layer)
}

const WALK_TILES: usize = 5;
const WALK_STEP_M: f32 = 120.0;

fn walk_hosts() -> Vec<ChicoGroveHost> {
	let mut hosts = Vec::new();
	for (i, (_layering, layers)) in walk_layerings().into_iter().enumerate() {
		let origin = Vec3::new(0.0, 0.0, i as f32 * WALK_STEP_M);
		for layer in ForestLayer::ALL {
			if let Some(kind) = layer.kind(layers) {
				hosts.push(grow_host(kind, layer, origin));
			}
		}
	}
	hosts
}

fn walk_layerings() -> Vec<(LayeringKind, SelectedLayers)> {
	let selection = ForestSelection::default();
	let ix = ForestExtent::cell_index_containing(Vec3::new(-4950.0, 0.0, 0.0)).0;
	let mut out = Vec::new();
	let mut seen = std::collections::HashSet::new();
	for iz in -6..=8 {
		let forest = ForestExtent::from_cell_index(ix, iz);
		let layers = selection.layers_for(forest);
		if ForestLayer::ALL.iter().all(|layer| layer.kind(layers).is_none()) {
			continue;
		}
		let kind = select_layering(selection.noise, forest.center());
		if !seen.insert(kind) {
			continue;
		}
		out.push((kind, layers));
		if out.len() >= WALK_TILES {
			break;
		}
	}
	out
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

fn replace_forest_ground(world: &mut World, size: Vec2, center: Vec3) {
	let ids: Vec<_> = world.query_filtered::<Entity, With<ForestGround>>().iter(world).collect();
	for id in ids {
		world.entity_mut(id).despawn();
	}
	ensure_forest_ground_sized(world, size, center);
}

fn ensure_forest_ground(world: &mut World) {
	ensure_forest_ground_sized(world, Vec2::new(520.0, 860.0), Vec3::new(50.0, -0.05, 240.0));
}

fn ensure_forest_ground_sized(world: &mut World, size: Vec2, center: Vec3) {
	let existing = world.query_filtered::<Entity, With<ForestGround>>().iter(world).next();
	if existing.is_some() {
		return;
	}
	let mesh = {
		let mut meshes = world.resource_mut::<Assets<Mesh>>();
		meshes.add(Plane3d::default().mesh().size(size.x, size.y))
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
		Transform::from_translation(center),
		bevy::light::NotShadowCaster,
	));
}

#[derive(Component)]
struct ForestGround;

/// Cameras sit outside the 2×2 canopy: southwest vista, south edge, east floor.
pub const VISTA: (Vec3, Vec3) = (Vec3::new(-48.0, 26.0, -56.0), Vec3::new(90.0, 10.0, 90.0));
pub const EDGE: (Vec3, Vec3) = (Vec3::new(50.0, 12.0, 228.0), Vec3::new(70.0, 8.0, 90.0));
pub const FLOOR: (Vec3, Vec3) = (Vec3::new(248.0, 2.2, 12.0), Vec3::new(220.0, 0.6, 70.0));
/// Southwest of the 0 m tile, looking through 200 m–1 km of stacked groves.
pub const FAR: (Vec3, Vec3) = (Vec3::new(-90.0, 48.0, -180.0), Vec3::new(80.0, 6.0, 520.0));
pub const FAR_CLIP_M: f32 = 2000.0;

/// Hopscotch walk strip: start / mid / end, looking into each 100 m tile from the south edge.
pub const WALK_START: (Vec3, Vec3) = (Vec3::new(50.0, 2.8, -8.0), Vec3::new(50.0, 1.0, 48.0));
pub const WALK_MID: (Vec3, Vec3) = (Vec3::new(50.0, 2.8, 232.0), Vec3::new(50.0, 1.0, 288.0));
pub const WALK_END: (Vec3, Vec3) = (Vec3::new(50.0, 2.8, 472.0), Vec3::new(50.0, 1.0, 528.0));

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cameras_sit_outside_the_canopy_footprint() {
		assert!(VISTA.0.x < 0.0 && VISTA.0.z < 0.0);
		assert!(EDGE.0.z > 200.0);
		assert!(FLOOR.0.x > 200.0 && FLOOR.0.z < 20.0);
		assert!(FAR.0.z < 0.0 && FAR.0.y > 30.0);
		assert!((FAR.1.z - FAR.0.z) > 600.0);
		assert!(WALK_START.0.z < 0.0 && WALK_START.0.y < 5.0);
		assert!(WALK_MID.0.z > 220.0 && WALK_MID.0.z < 240.0);
		assert!(WALK_END.0.z > 460.0 && WALK_END.0.z < 480.0);
	}

	#[test]
	fn walk_strip_covers_multiple_hopscotch_layerings() {
		let layerings = walk_layerings();
		assert!(layerings.len() >= 3, "got {} layerings", layerings.len());
		let unique: std::collections::HashSet<_> =
			layerings.iter().map(|(kind, _)| *kind).collect();
		assert_eq!(unique.len(), layerings.len());
	}

	#[test]
	fn far_hosts_cover_the_one_km_present_ring() {
		GroveGpuVisuals::admit(true);
		let hosts = far_hosts();
		let max_z = hosts.iter().fold(0.0_f32, |max_z, host| {
			host.tile
				.plant_sites()
				.iter()
				.map(|site| site.root.z)
				.fold(max_z, f32::max)
		});
		GroveGpuVisuals::admit(false);
		assert_eq!(hosts.len(), 12);
		assert!(max_z >= 900.0, "need a 900 m tile, max_z={max_z}");
	}

	#[test]
	fn walk_hosts_place_plants_on_the_playground_strip() {
		GroveGpuVisuals::admit(true);
		let hosts = walk_hosts();
		let sites: usize = hosts.iter().map(|host| host.tile.plant_sites().len()).sum();
		let gpu: usize = hosts
			.iter()
			.map(|host| {
				let visuals = host.gpu_visuals();
				visuals.trees.as_ref().map(|grove| grove.0.plants.len()).unwrap_or(0)
					+ visuals.bushes.as_ref().map(|patch| patch.samples.len()).unwrap_or(0)
					+ visuals.grass.as_ref().map(|patch| patch.samples.len()).unwrap_or(0)
			})
			.sum();
		GroveGpuVisuals::admit(false);
		assert!(sites > 100, "walk strip should be planted, sites={sites}");
		assert_eq!(gpu, sites, "GPU samples should match planted sites");
	}
}
