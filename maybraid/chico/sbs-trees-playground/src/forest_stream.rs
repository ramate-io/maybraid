//! Rough `/show forest` streaming: keep a ring of 1600 m cells as grove hosts.

use std::collections::HashMap;

use bevy::prelude::*;
use chico_forests::{
	select_cell, ChicoForest, ForestExtent, ForestGroveTile, LayeringKind, NeighborLayers,
	SelectedLayers,
};
use chico_groves::{
	Alpine, AridConiferSapling, BraidGrass, BushScrub, ChristmasTaiga, CommonTufts,
	ConiferMassives, ConiferSapling, DateGrove, Dryland, FlatTerrainSample, ForlornSavanna,
	GoettingenFollow, HighBush, JerrysChaparral, JungleLowerMassives, JungleMassives, Leeward,
	LevantineScrub, LowBush, MonsterGrass, Orchard, PalmShade, RiparianGeneral, RiparianMix,
	RiverineGreen, RollingOaks, Shamanhome, SpottyBushes, Storytellers, StrangeOasis, TallGrass,
	TemperateLowerMassives, TemperateMassives, TradeWinds, TropicalThicket, TropicalTufts,
	TropicalUndergrowth, UnendingJungle, Vineyard, WanderingAcacia, WildGrass,
};
use chico_vegetation_components::{spawn_lod_scene_host, vegetation_bounds, VegetationComponents};
use lod::gen::LodScene;
use procedural_common::NoiseParams;

use crate::camera::CameraController;
use crate::commands::show::{ShowConfig, ShowRoot, ShowSubject};
use crate::ground::GroundPlane;

/// Default Chebyshev ring around the camera cell (`3 × 3` when radius is 1).
pub const DEFAULT_FOREST_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_FOREST_NOISE: &str = "1337,0.0005,1,1";

/// Clap parser for a well-known layering kebab name.
pub fn parse_layering_kind(name: &str) -> Result<LayeringKind, String> {
	LayeringKind::from_kebab(name).ok_or_else(|| {
		let names: Vec<_> = LayeringKind::ALL.iter().map(|kind| kind.as_kebab()).collect();
		format!("unknown layering {name:?}; expected one of: {}", names.join(", "))
	})
}

const FOREST_CAMERA_SPEED: f32 = 80.0;
const DEFAULT_CAMERA_SPEED: f32 = 18.0;
/// Stay on the current stream cell until the camera is this far into a neighbor.
const STREAM_COMMIT_MARGIN: f32 = 80.0;

fn spawn_grove_host<T>(commands: &mut Commands, grove: &T) -> Vec<Entity>
where
	T: LodScene + VegetationComponents + Component + Clone + Send + Sync + 'static,
{
	let bounds = grove
		.structural_lod()
		.map(|p| p.footprint_aabb())
		.unwrap_or_else(|| vegetation_bounds(grove));
	let entities = spawn_lod_scene_host(commands, grove, Transform::IDENTITY, bounds);
	for entity in &entities {
		commands.entity(*entity).insert(ShowRoot);
	}
	entities
}

fn spawn_forest_grove_tile(commands: &mut Commands, tile: &ForestGroveTile) -> Vec<Entity> {
	match tile {
		ForestGroveTile::Alpine(g) => spawn_grove_host::<Alpine>(commands, g),
		ForestGroveTile::AridConiferSapling(g) => {
			spawn_grove_host::<AridConiferSapling>(commands, g)
		}
		ForestGroveTile::BraidGrass(g) => spawn_grove_host::<BraidGrass>(commands, g),
		ForestGroveTile::BushScrub(g) => spawn_grove_host::<BushScrub>(commands, g),
		ForestGroveTile::ChristmasTaiga(g) => spawn_grove_host::<ChristmasTaiga>(commands, g),
		ForestGroveTile::CommonTufts(g) => spawn_grove_host::<CommonTufts>(commands, g),
		ForestGroveTile::ConiferMassives(g) => spawn_grove_host::<ConiferMassives>(commands, g),
		ForestGroveTile::ConiferSapling(g) => spawn_grove_host::<ConiferSapling>(commands, g),
		ForestGroveTile::DateGrove(g) => spawn_grove_host::<DateGrove>(commands, g),
		ForestGroveTile::Dryland(g) => spawn_grove_host::<Dryland>(commands, g),
		ForestGroveTile::ForlornSavanna(g) => spawn_grove_host::<ForlornSavanna>(commands, g),
		ForestGroveTile::GoettingenFollow(g) => spawn_grove_host::<GoettingenFollow>(commands, g),
		ForestGroveTile::HighBush(g) => spawn_grove_host::<HighBush>(commands, g),
		ForestGroveTile::JerrysChaparral(g) => spawn_grove_host::<JerrysChaparral>(commands, g),
		ForestGroveTile::JungleLowerMassives(g) => {
			spawn_grove_host::<JungleLowerMassives>(commands, g)
		}
		ForestGroveTile::JungleMassives(g) => spawn_grove_host::<JungleMassives>(commands, g),
		ForestGroveTile::Leeward(g) => spawn_grove_host::<Leeward>(commands, g),
		ForestGroveTile::LevantineScrub(g) => spawn_grove_host::<LevantineScrub>(commands, g),
		ForestGroveTile::LowBush(g) => spawn_grove_host::<LowBush>(commands, g),
		ForestGroveTile::MonsterGrass(g) => spawn_grove_host::<MonsterGrass>(commands, g),
		ForestGroveTile::Orchard(g) => spawn_grove_host::<Orchard>(commands, g),
		ForestGroveTile::PalmShade(g) => spawn_grove_host::<PalmShade>(commands, g),
		ForestGroveTile::RiparianGeneral(g) => spawn_grove_host::<RiparianGeneral>(commands, g),
		ForestGroveTile::RiparianMix(g) => spawn_grove_host::<RiparianMix>(commands, g),
		ForestGroveTile::RiverineGreen(g) => spawn_grove_host::<RiverineGreen>(commands, g),
		ForestGroveTile::RollingOaks(g) => spawn_grove_host::<RollingOaks>(commands, g),
		ForestGroveTile::Shamanhome(g) => spawn_grove_host::<Shamanhome>(commands, g),
		ForestGroveTile::SpottyBushes(g) => spawn_grove_host::<SpottyBushes>(commands, g),
		ForestGroveTile::Storytellers(g) => spawn_grove_host::<Storytellers>(commands, g),
		ForestGroveTile::StrangeOasis(g) => spawn_grove_host::<StrangeOasis>(commands, g),
		ForestGroveTile::TallGrass(g) => spawn_grove_host::<TallGrass>(commands, g),
		ForestGroveTile::TemperateLowerMassives(g) => {
			spawn_grove_host::<TemperateLowerMassives>(commands, g)
		}
		ForestGroveTile::TemperateMassives(g) => spawn_grove_host::<TemperateMassives>(commands, g),
		ForestGroveTile::TradeWinds(g) => spawn_grove_host::<TradeWinds>(commands, g),
		ForestGroveTile::TropicalThicket(g) => spawn_grove_host::<TropicalThicket>(commands, g),
		ForestGroveTile::TropicalTufts(g) => spawn_grove_host::<TropicalTufts>(commands, g),
		ForestGroveTile::TropicalUndergrowth(g) => {
			spawn_grove_host::<TropicalUndergrowth>(commands, g)
		}
		ForestGroveTile::UnendingJungle(g) => spawn_grove_host::<UnendingJungle>(commands, g),
		ForestGroveTile::Vineyard(g) => spawn_grove_host::<Vineyard>(commands, g),
		ForestGroveTile::WanderingAcacia(g) => spawn_grove_host::<WanderingAcacia>(commands, g),
		ForestGroveTile::WildGrass(g) => spawn_grove_host::<WildGrass>(commands, g),
	}
}

fn layers_for(
	ix: i32,
	iz: i32,
	noise: NoiseParams,
	layering: Option<LayeringKind>,
) -> SelectedLayers {
	match layering {
		Some(kind) => kind.layering().typical_layers(),
		None => select_cell(ForestExtent::from_cell_index(ix, iz), noise),
	}
}

fn neighbor_layers_from(
	cache: &HashMap<(i32, i32), SelectedLayers>,
	ix: i32,
	iz: i32,
) -> NeighborLayers {
	NeighborLayers {
		north: cache.get(&(ix, iz + 1)).copied(),
		east: cache.get(&(ix + 1, iz)).copied(),
		south: cache.get(&(ix, iz - 1)).copied(),
		west: cache.get(&(ix - 1, iz)).copied(),
	}
}

fn spawn_forest_cell(
	commands: &mut Commands,
	ix: i32,
	iz: i32,
	layers: SelectedLayers,
	neighbors: NeighborLayers,
) -> Vec<Entity> {
	let extent = ForestExtent::from_cell_index(ix, iz);
	let assembled =
		chico_forests::assemble(extent, layers, neighbors, &FlatTerrainSample::default());
	let forest = ChicoForest { extent, assembled };
	let layers = forest.assembled.layers;
	info!(
		"forest cell ({ix},{iz}) layering={:?} tufts={:?} understory={:?} lower={:?} upper={:?} tiles={}",
		layers.layering,
		layers.tufts,
		layers.understory,
		layers.lower_canopy,
		layers.upper_canopy,
		forest.tiles().count()
	);
	let mut entities = Vec::new();
	for tile in forest.tiles() {
		entities.extend(spawn_forest_grove_tile(commands, tile));
	}
	entities
}

/// Keep a Chebyshev ring of assembled forest cells around the camera.
///
/// Grows at most one missing cell per frame. `sync_show` clears [`ShowRoot`]s when
/// the subject key changes; this system only tracks live cell indices.
pub fn stream_forest(
	mut commands: Commands,
	config: Res<ShowConfig>,
	camera: Query<&Transform, With<Camera3d>>,
	mut controller: Query<&mut CameraController, With<Camera3d>>,
	mut ground: Query<&mut Transform, (With<GroundPlane>, Without<Camera3d>)>,
	mut live: Local<HashMap<(i32, i32), Vec<Entity>>>,
	mut selected: Local<HashMap<(i32, i32), SelectedLayers>>,
	mut last_key: Local<Option<String>>,
	mut forest_camera: Local<bool>,
	mut stream_center: Local<Option<(i32, i32)>>,
) {
	let Some(ShowSubject::Forest { noise, stream_radius, layering }) = &config.subject else {
		if *forest_camera {
			if let Ok(mut ctrl) = controller.single_mut() {
				ctrl.speed = DEFAULT_CAMERA_SPEED;
			}
			*forest_camera = false;
		}
		live.clear();
		selected.clear();
		last_key.take();
		stream_center.take();
		return;
	};

	if !*forest_camera {
		if let Ok(mut ctrl) = controller.single_mut() {
			ctrl.speed = FOREST_CAMERA_SPEED;
		}
		*forest_camera = true;
	}

	let layering_key = layering.map(LayeringKind::as_kebab).unwrap_or("hopscotch");
	let key = format!("forest:{layering_key}|{noise:?}|r={stream_radius}");
	if last_key.as_ref() != Some(&key) {
		live.clear();
		selected.clear();
		stream_center.take();
		*last_key = Some(key);
	}

	let Ok(cam) = camera.single() else {
		return;
	};
	if let Ok(mut ground_tf) = ground.single_mut() {
		ground_tf.translation.x = cam.translation.x;
		ground_tf.translation.z = cam.translation.z;
	}

	let raw = ForestExtent::cell_index_containing(cam.translation);
	let center = match *stream_center {
		Some(current) => {
			ForestExtent::cell_index_committed(cam.translation, current, STREAM_COMMIT_MARGIN)
		}
		None => raw,
	};
	*stream_center = Some(center);
	let wanted: Vec<(i32, i32)> = ForestExtent::cell_ring(center, *stream_radius).collect();
	// Select halo (R+1) without growing it so neighbor grove slots have recipes.
	let halo: Vec<(i32, i32)> =
		ForestExtent::cell_ring(center, stream_radius.saturating_add(1)).collect();
	for &(sx, sz) in &halo {
		selected
			.entry((sx, sz))
			.or_insert_with(|| layers_for(sx, sz, *noise, *layering));
	}
	selected.retain(|cell, _| halo.contains(cell));

	live.retain(|(ix, iz), entities| {
		if wanted.contains(&(*ix, *iz)) {
			true
		} else {
			for entity in entities.drain(..) {
				commands.entity(entity).despawn();
			}
			false
		}
	});

	let mut missing: Vec<(i32, i32)> =
		wanted.into_iter().filter(|cell| !live.contains_key(cell)).collect();
	missing.sort_by_key(|(ix, iz)| (ix - center.0).unsigned_abs() + (iz - center.1).unsigned_abs());
	let Some(&(ix, iz)) = missing.first() else {
		return;
	};
	let layers = selected
		.get(&(ix, iz))
		.copied()
		.unwrap_or_else(|| layers_for(ix, iz, *noise, *layering));
	let neighbors = neighbor_layers_from(&selected, ix, iz);
	let entities = spawn_forest_cell(&mut commands, ix, iz, layers, neighbors);
	live.insert((ix, iz), entities);
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use procedural_common::noise_params_from_scalar_str;

	#[test]
	fn default_forest_noise_parses() -> Result<()> {
		let noise = noise_params_from_scalar_str(DEFAULT_FOREST_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(noise.seed, 1337);
		assert!((noise.frequency - 0.0005).abs() < 1e-8);
		Ok(())
	}

	#[test]
	fn parse_layering_kind_accepts_kebab() -> Result<()> {
		assert_eq!(
			parse_layering_kind("ag-town").map_err(|e| anyhow::anyhow!("{e}"))?,
			LayeringKind::AgTown
		);
		assert!(parse_layering_kind("not-a-forest").is_err());
		Ok(())
	}

	#[test]
	fn halo_is_display_ring_plus_one() -> Result<()> {
		let r = DEFAULT_FOREST_STREAM_RADIUS;
		let display: Vec<_> = ForestExtent::cell_ring((0, 0), r).collect();
		let halo: Vec<_> = ForestExtent::cell_ring((0, 0), r.saturating_add(1)).collect();
		assert_eq!(display.len(), 9);
		assert_eq!(halo.len(), 25);
		assert!(display.iter().all(|cell| halo.contains(cell)));
		Ok(())
	}

	#[test]
	fn neighbor_layers_from_reads_cardinal_cache() -> Result<()> {
		let east = SelectedLayers {
			layering: LayeringKind::MiRobles,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: Some(chico_forests::ForestGroveKind::RollingOaks),
		};
		let mut cache = HashMap::new();
		cache.insert((1, 0), east);
		let neighbors = neighbor_layers_from(&cache, 0, 0);
		assert_eq!(neighbors.east, Some(east));
		assert!(neighbors.north.is_none());
		assert!(neighbors.south.is_none());
		assert!(neighbors.west.is_none());
		Ok(())
	}
}
