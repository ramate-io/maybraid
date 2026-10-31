use crate::terrain_height_at;
use bevy::{ecs::system::RunSystemOnce, prelude::*};
use durham::{
	BaseTerrainNoise, DurhamSurface, Terrain, TerrainCellLayout, TerrainConfig, WorldBaseTerrain,
};
use lod::{gen::Id, hcsg::HcsgStorage};
use richmond::{PadComplex, PaddedTerrain, TerrainWithPads};
use std::{
	sync::{mpsc, Arc},
	time::Duration,
};

type Ground = terrain_layer_model::OnTerrain<durham::Durham>;

fn world() -> (World, Terrain) {
	let mut world = World::new();
	let layout = TerrainCellLayout::default();
	let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
	let terrain = Terrain::base_cell_for_test(&layout, 0, 0, base.clone());
	world.insert_resource(HcsgStorage::default());
	world.insert_resource(layout);
	world.insert_resource(WorldBaseTerrain(base));
	(world, terrain)
}

fn height(world: &mut World) -> anyhow::Result<Option<f32>> {
	world
		.run_system_once(|surface: DurhamSurface, storage: Res<HcsgStorage>| {
			terrain_height_at(&surface, Some(&storage), Vec2::splat(8.0))
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))
}

#[test]
fn published_padded_height_precedes_raw_durham_and_missing_cells_wait() -> anyhow::Result<()> {
	let (mut world, terrain) = world();
	assert_eq!(height(&mut world)?, None);
	let cell = terrain.cell;
	world
		.resource::<HcsgStorage>()
		.publish(Id::from_cell(cell), Arc::new(terrain.clone()), cell);
	let raw = height(&mut world)?.ok_or_else(|| anyhow::anyhow!("Durham height missing"))?;
	let pad_y = raw - 40.0;
	let pad = PadComplex::building_skirt(
		Vec2::splat(8.0),
		Vec2::splat(20.0),
		0.0,
		pad_y,
		Default::default(),
	);
	let padded = TerrainWithPads::compose(&terrain, [&pad]);
	world.resource::<HcsgStorage>().publish(
		Id::from_cell(cell),
		Arc::new(PaddedTerrain::<Ground>::new(padded)),
		cell,
	);
	let sampled = height(&mut world)?.ok_or_else(|| anyhow::anyhow!("padded height missing"))?;
	assert!((sampled - pad_y).abs() < 1e-3);
	assert!(sampled < raw - 30.0);
	Ok(())
}

#[test]
fn a_busy_durham_store_stays_pending_then_retries_published_height() -> anyhow::Result<()> {
	let (mut world, terrain) = world();
	let cell = terrain.cell;
	world
		.resource::<HcsgStorage>()
		.publish(Id::from_cell(cell), Arc::new(terrain.clone()), cell);
	let storage = world.resource::<HcsgStorage>().clone();
	let (locked_tx, locked_rx) = mpsc::channel();
	let (release_tx, release_rx) = mpsc::channel();
	let worker = std::thread::spawn(move || {
		storage.publish_unless(Id::from_cell(cell), Arc::new(terrain), cell, || {
			if locked_tx.send(()).is_err() {
				return true;
			}
			release_rx.recv().is_err()
		})
	});
	locked_rx.recv_timeout(Duration::from_secs(5))?;
	let blocked = height(&mut world);
	release_tx.send(())?;
	worker.join().map_err(|_| anyhow::anyhow!("publication worker failed"))?;
	assert_eq!(blocked?, None, "busy storage must not substitute a stale fallback");
	assert!(height(&mut world)?.is_some());
	Ok(())
}
