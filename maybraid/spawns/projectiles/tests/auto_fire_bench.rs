//! Headless A/B: main `spawn_flight_aged` (mesh in sim) vs batched
//! `SpawnProjectile` / `spawn_flight_sim`.
//!
//! Run:
//! ```bash
//! nix develop --command cargo test -p maybraid-spawns-projectiles --release auto_fire_bench -- --ignored --nocapture
//! ```

use std::hint::black_box;

use anyhow::Result;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use gimme_bench_support::{AbReport, Microbench, PassLifetime};
use maybraid_spawns_projectiles::{ProjectileSpec, SpawnProjectile, realize_projectiles};
use projectiles::{BoltSpec, Flight, ProjectileVisualCache, spawn_flight_aged};

const SHOTS: u32 = 240;
const REPEATS: usize = 7;

fn launch(index: u32) -> SpawnProjectile {
	SpawnProjectile::launch(
		Entity::from_bits(2),
		Vec3::new(index as f32 * 0.1, 1.0, 0.0),
		Vec3::Z,
		ProjectileSpec::Bolt(BoltSpec::default()),
		0.0,
		None,
	)
}

fn aged_world() -> World {
	let mut world = World::new();
	world.init_resource::<Assets<Mesh>>();
	world.init_resource::<Assets<StandardMaterial>>();
	world.init_resource::<ProjectileVisualCache>();
	world
}

fn sim_world() -> World {
	let mut world = World::new();
	world.init_resource::<Messages<SpawnProjectile>>();
	world
}

fn spawn_aged_burst(world: &mut World) -> Result<usize> {
	world
		.run_system_once(
			|mut commands: Commands,
			 mut meshes: ResMut<Assets<Mesh>>,
			 mut materials: ResMut<Assets<StandardMaterial>>,
			 mut visuals: ResMut<ProjectileVisualCache>| {
				for index in 0..SHOTS {
					let request = launch(index);
					let (length, radius, speed, max_range, penetration, max_age, color, gravity) =
						request.spec.ballistic();
					spawn_flight_aged(
						&mut commands,
						&mut meshes,
						&mut materials,
						&mut visuals,
						request.origin,
						request.direction,
						length,
						radius,
						speed,
						max_range,
						penetration,
						max_age,
						color,
						gravity,
						request.age,
					);
				}
			},
		)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	Ok(world.query::<&Flight>().iter(world).count())
}

fn spawn_sim_burst(world: &mut World) -> Result<(usize, std::time::Duration)> {
	{
		let mut messages = world.resource_mut::<Messages<SpawnProjectile>>();
		for index in 0..SHOTS {
			messages.write(launch(index));
		}
	}
	let started = std::time::Instant::now();
	world
		.run_system_once(realize_projectiles)
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	let latency = started.elapsed();
	Ok((world.query::<&Flight>().iter(world).count(), latency))
}

#[test]
#[ignore = "release micro-benchmark; run with --release --ignored --nocapture"]
fn auto_fire_bench() -> Result<()> {
	let bench = Microbench { warmup: 1, repeats: REPEATS };
	let mut original_allocs = 0usize;
	let mut revised_allocs = 0usize;
	let mut latencies = Vec::new();
	let (original, revised) = bench.collect_ab_alternating(
		|| {
			let mut world = aged_world();
			let flights = spawn_aged_burst(&mut world)?;
			original_allocs = world.resource::<Assets<Mesh>>().len()
				+ world.resource::<Assets<StandardMaterial>>().len()
				+ flights;
			assert_eq!(flights, SHOTS as usize);
			black_box(flights);
			Ok(())
		},
		|| {
			let mut world = sim_world();
			let (flights, latency) = spawn_sim_burst(&mut world)?;
			revised_allocs = flights;
			latencies.push(latency);
			assert_eq!(flights, SHOTS as usize);
			black_box(flights);
			Ok(())
		},
	)?;
	AbReport {
		label: "sustained automatic fire",
		original_name: "main spawn_flight_aged",
		revised_name: "batched SpawnProjectile",
		original: &original,
		revised: &revised,
		operations: SHOTS,
		operation_name: "shot",
		lifetime: PassLifetime::FreshPerPass,
		notes: "headless; mesh assets stay out of the simulation path",
	}
	.write_to_stderr()?;
	let mean_latency = latencies.iter().copied().sum::<std::time::Duration>()
		/ u32::try_from(latencies.len().max(1))?;
	eprintln!(
		"auto_fire_bench allocations (mesh+material+entities original / entities revised): \
		 {original_allocs} / {revised_allocs}; request-to-realization mean {mean_latency:?} \
		 for {SHOTS} shots"
	);
	Ok(())
}
