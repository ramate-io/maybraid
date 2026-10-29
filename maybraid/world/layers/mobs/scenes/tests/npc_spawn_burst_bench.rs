//! Headless dense NPC spawn burst. Both sides call `character.spawn`
//! (the current main / cell-batch path). The original extra cost is the
//! request-identity bookkeeping the population extract would have added.
//!
//! Run:
//! ```bash
//! nix develop --command cargo test -p mob-scenes --release npc_spawn_burst -- --ignored --nocapture
//! ```

use std::hint::black_box;

use anyhow::Result;
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use gimme_bench_support::{AbReport, Microbench, PassLifetime};
use mob_scenes::{MobKind, MobRosterRecipe};

const HOSTS: u32 = 16;
const REPEATS: usize = 7;

fn roster() -> MobRosterRecipe {
	MobRosterRecipe::from_kind(MobKind::Pleb, 12.0, 8.0)
}

fn spawn_direct(world: &mut World, roster: &MobRosterRecipe) -> Result<usize> {
	let roster = roster.clone();
	world
		.run_system_once(move |mut commands: Commands| {
			let mut spawned = 0usize;
			for host in 0..HOSTS {
				let origin = Vec3::new(host as f32 * 6.0, 0.0, 0.0);
				for member in &roster.members {
					member
						.character
						.spawn(&mut commands, Transform::from_translation(origin + member.offset));
					spawned += 1;
				}
			}
			spawned
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))
}

fn spawn_tracked(world: &mut World, roster: &MobRosterRecipe) -> Result<usize> {
	let roster = roster.clone();
	world
		.run_system_once(move |mut commands: Commands| {
			let mut pending: Vec<(u64, Vec3)> = Vec::new();
			let mut request = 1u64;
			for host in 0..HOSTS {
				let origin = Vec3::new(host as f32 * 6.0, 0.0, 0.0);
				for member in &roster.members {
					pending.push((request, origin + member.offset));
					request += 1;
				}
			}
			let mut spawned = 0usize;
			for (index, pose) in pending {
				let member = &roster.members[index as usize % roster.members.len()];
				member.character.spawn(&mut commands, Transform::from_translation(pose));
				spawned += 1;
			}
			spawned
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))
}

#[test]
#[ignore = "release micro-benchmark; run with --release --ignored --nocapture"]
fn npc_spawn_burst() -> Result<()> {
	let roster = roster();
	let members = roster.members.len();
	assert!(members > 0, "Pleb roster must have members");
	let operations = HOSTS * u32::try_from(members)?;
	let bench = Microbench { warmup: 1, repeats: REPEATS };
	let (original, revised) = bench.collect_ab_alternating(
		|| {
			let mut world = World::new();
			let spawned = spawn_tracked(&mut world, &roster)?;
			assert_eq!(spawned, operations as usize);
			black_box(spawned);
			Ok(())
		},
		|| {
			let mut world = World::new();
			let spawned = spawn_direct(&mut world, &roster)?;
			assert_eq!(spawned, operations as usize);
			black_box(spawned);
			Ok(())
		},
	)?;
	AbReport {
		label: "dense NPC spawn burst",
		original_name: "identity-tracked requests then spawn",
		revised_name: "direct character.spawn (main)",
		original: &original,
		revised: &revised,
		operations,
		operation_name: "member",
		lifetime: PassLifetime::FreshPerPass,
		notes: "cell-level batch preserved; no population crate",
	}
	.write_to_stderr()?;
	eprintln!("npc_spawn_burst hosts={HOSTS} members_per_host={members} entities={operations}");
	Ok(())
}
