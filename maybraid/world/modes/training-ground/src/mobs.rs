//! Training fields its FFA roster as one mob cell on the courtyard.

use barking::{Barking, BarkingConfig, MobCellWrites};
use bevy::prelude::*;
use chico::Chico;
use durham::Durham;
use layer_stack::{ActiveGenerationMode, GenerationModeSystems};
use lod::gen::Id;
use lod::{LodGenerateSystems, LodPresentSystems};
use mob_layer_model::{MobCellPresented, MobGenerationSystems, MobScheme};
use richmond::Richmond;
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

use crate::arena::{publish_training_arena, TrainingArena};
use crate::{TrainingGround, TrainingMap};

/// Training brawler mob host. Its members carry `MemberOf` back to it.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TrainingBrawler;

#[derive(Resource, Debug)]
struct TrainingRoster {
	map: TrainingMap,
	mob_id: Id,
}

impl MobScheme<Barking<Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>>>
	for TrainingGround
{
	fn install(app: &mut App, _config: &BarkingConfig) {
		crate::arena::install(app);
		app.add_systems(
			Update,
			write_training_roster
				.in_set(GenerationModeSystems::<TrainingGround>::default())
				.in_set(MobGenerationSystems)
				.after(publish_training_arena)
				.before(LodGenerateSystems::Produce)
				.before(LodPresentSystems::Produce),
		);
		app.add_systems(
			Update,
			tag_training_brawlers
				.in_set(GenerationModeSystems::<TrainingGround>::default())
				.after(LodPresentSystems::Produce),
		);
		app.add_systems(
			OnExit(ActiveGenerationMode::of::<TrainingGround>()),
			clear_training_roster,
		);
	}
}

fn clear_training_roster(
	written: Option<Res<TrainingRoster>>,
	mut cells: MobCellWrites,
	mut commands: Commands,
) {
	if let Some(written) = written {
		cells.remove(written.mob_id);
	}
	commands.remove_resource::<TrainingRoster>();
}

fn write_training_roster(
	arena: Option<Res<TrainingArena>>,
	written: Option<Res<TrainingRoster>>,
	mut cells: MobCellWrites,
	mut commands: Commands,
) {
	let Some(arena) = arena.as_deref() else {
		if let Some(written) = written.as_deref() {
			cells.remove(written.mob_id);
			commands.remove_resource::<TrainingRoster>();
		}
		return;
	};
	if let Some(written) = written.as_deref() {
		if written.map == arena.map() {
			return;
		}
		cells.remove(written.mob_id);
		commands.remove_resource::<TrainingRoster>();
	}
	let cell = arena.mob_cell();
	let mob_id = cells.insert(cell);
	commands.insert_resource(TrainingRoster { map: arena.map(), mob_id });
}

fn tag_training_brawlers(
	roster: Option<Res<TrainingRoster>>,
	mut presented: MessageReader<MobCellPresented>,
	mut commands: Commands,
) {
	let Some(roster) = roster else {
		return;
	};
	for message in presented.read() {
		if message.id != roster.mob_id {
			continue;
		}
		for host in &message.hosts {
			commands.entity(*host).insert(TrainingBrawler);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use barking::{GroupKind, MobCell, MobIndex};
	use bevy::ecs::message::Messages;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::math::bounding::Aabb3d;
	use bevy::prelude::{App, MessageReader, MessageWriter, MinimalPlugins, NextState, World};
	use bevy::state::app::StatesPlugin;
	use layer_stack::{GenerationMode, GenerationModePlugin};
	use lod::gen::{LodGenerated, SpatialIndex};
	use richmond::{DevelopmentCell, DevelopmentConfig, DevelopmentEntryStore};

	use crate::arena::publish_training_arena;
	use crate::{training_development_cell, TrainingPlazaStamped, TrainingRound};

	struct OtherMode;

	impl GenerationMode for OtherMode {}

	const TRAINING_ROSTER: usize = 16;

	fn stamp_for(round: TrainingRound, center: Vec2, footprint: Vec2) -> TrainingPlazaStamped {
		let cell = training_development_cell(center);
		TrainingPlazaStamped::new(
			round,
			Id::from_cell(cell),
			Vec::new(),
			center,
			footprint + Vec2::splat(16.0),
			footprint,
			4.0,
		)
	}

	fn roster_world(round: TrainingRound, center: Vec2, footprint: Vec2) -> anyhow::Result<World> {
		let mut world = World::new();
		world.init_resource::<Messages<LodGenerated<MobCell>>>();
		world.insert_resource(MobIndex::default());
		let cell = training_development_cell(center);
		let config = DevelopmentConfig::from_world_seed(round.development_seed());
		let filled = DevelopmentCell::with_les_halles(cell, 4.0, &config);
		let built = filled
			.built(config.seed as i32)
			.ok_or_else(|| anyhow::anyhow!("les halles built"))?;
		let mut store = DevelopmentEntryStore::default();
		let cell_id = Id::from_cell(cell);
		store.insert_cell(cell_id, filled);
		store.insert_built(cell_id, built, cell);
		world.insert_resource(store);
		world.insert_resource(stamp_for(round, center, footprint));
		Ok(world)
	}

	fn publish_then_write(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(write_training_roster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn the_scheme_writes_one_cell_for_the_arena_squads() -> anyhow::Result<()> {
		let round = TrainingRound::new(42);
		let mut world = roster_world(round, Vec2::ZERO, Vec2::splat(36.0))?;
		publish_then_write(&mut world)?;
		let mob_id = world
			.get_resource::<TrainingRoster>()
			.ok_or_else(|| anyhow::anyhow!("roster was not written"))?
			.mob_id;
		let cell = world
			.resource::<MobIndex>()
			.get(mob_id)
			.ok_or_else(|| anyhow::anyhow!("mob cell missing"))?
			.clone();
		let arena = world
			.get_resource::<TrainingArena>()
			.ok_or_else(|| anyhow::anyhow!("arena missing"))?
			.clone();
		anyhow::ensure!(cell.extent.id() == mob_id);
		let (lo, hi) = arena.xz_bounds();
		let bounds = cell.extent.aabb();
		anyhow::ensure!(bounds.min.x <= lo.x && bounds.max.x >= hi.x);
		anyhow::ensure!(bounds.min.z <= lo.y && bounds.max.z >= hi.y);
		anyhow::ensure!(cell.groups.iter().all(|group| group.kind == GroupKind::Placed));
		anyhow::ensure!(cell.groups.iter().all(|group| group.kind != GroupKind::Frontier));
		anyhow::ensure!(cell
			.groups
			.iter()
			.all(|group| { (group.extent - arena.group_extent()).abs() < 1e-3 }));
		anyhow::ensure!(cell.groups.len() == arena.mob_cell().groups.len(), "one group per squad");
		let seats: usize = cell.groups.iter().map(|group| group.mobs.len()).sum();
		anyhow::ensure!(seats == arena.mob_cell().groups.len(), "one placed mob per squad");
		let roster_size: usize = cell
			.groups
			.iter()
			.flat_map(|group| group.mobs.iter().map(|placed| placed.scene.mob.roster.members.len()))
			.sum();
		anyhow::ensure!(roster_size == TRAINING_ROSTER, "roster size {roster_size}");
		let announced = world
			.run_system_once(|mut reader: MessageReader<LodGenerated<MobCell>>| {
				reader.read().map(|message| message.id).collect::<Vec<_>>()
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(
			announced == vec![mob_id],
			"writing the roster announces the cell so the presenter can enqueue it"
		);
		Ok(())
	}

	#[test]
	fn a_new_map_rewrites_the_cell_and_a_new_life_keeps_it() -> anyhow::Result<()> {
		let round = TrainingRound::new(42);
		let mut world = roster_world(round, Vec2::ZERO, Vec2::splat(36.0))?;
		publish_then_write(&mut world)?;
		let first = world.resource::<TrainingRoster>().mob_id;
		world.insert_resource(stamp_for(round.next_life(), Vec2::ZERO, Vec2::splat(36.0)));
		publish_then_write(&mut world)?;
		anyhow::ensure!(
			world.resource::<TrainingRoster>().mob_id == first,
			"a new life keeps the cell"
		);

		let next = round.next();
		let next_center = Vec2::new(400.0, 0.0);
		let next_cell = training_development_cell(next_center);
		let config = DevelopmentConfig::from_world_seed(next.development_seed());
		let filled = DevelopmentCell::with_les_halles(next_cell, 4.0, &config);
		let built =
			filled.built(config.seed as i32).ok_or_else(|| anyhow::anyhow!("next built"))?;
		let next_id = Id::from_cell(next_cell);
		{
			let mut store = world.resource_mut::<DevelopmentEntryStore>();
			store.insert_cell(next_id, filled);
			store.insert_built(next_id, built, next_cell);
		}
		world.insert_resource(stamp_for(next, next_center, Vec2::splat(36.0)));
		publish_then_write(&mut world)?;
		anyhow::ensure!(
			world.resource::<MobIndex>().get(first).is_none(),
			"a new map removes the previous cell"
		);
		anyhow::ensure!(world.resource::<TrainingRoster>().mob_id != first);
		Ok(())
	}

	#[test]
	fn leaving_training_leaves_the_index_empty() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			StatesPlugin,
			GenerationModePlugin::<TrainingGround>::initial(),
			GenerationModePlugin::<OtherMode>::default(),
		));
		<TrainingGround as MobScheme<
			Barking<Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>>,
		>>::install(&mut app, &BarkingConfig::default());
		app.init_resource::<Messages<LodGenerated<MobCell>>>();
		let round = TrainingRound::new(42);
		let mut src = roster_world(round, Vec2::ZERO, Vec2::splat(36.0))?;
		app.insert_resource(
			src.remove_resource::<MobIndex>().ok_or_else(|| anyhow::anyhow!("mob index"))?,
		);
		app.insert_resource(
			src.remove_resource::<DevelopmentEntryStore>()
				.ok_or_else(|| anyhow::anyhow!("developments"))?,
		);
		app.insert_resource(
			src.remove_resource::<TrainingPlazaStamped>()
				.ok_or_else(|| anyhow::anyhow!("stamp"))?,
		);
		app.world_mut()
			.run_system_once(publish_training_arena)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		app.world_mut()
			.run_system_once(write_training_roster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(!app.world().resource::<MobIndex>().is_empty());
		anyhow::ensure!(app.world().get_resource::<TrainingArena>().is_some());

		app.world_mut()
			.resource_mut::<NextState<ActiveGenerationMode>>()
			.set(ActiveGenerationMode::of::<OtherMode>());
		app.update();
		anyhow::ensure!(
			app.world().resource::<MobIndex>().is_empty(),
			"leaving Training drops the roster cell"
		);
		anyhow::ensure!(app.world().get_resource::<TrainingRoster>().is_none());
		anyhow::ensure!(app.world().get_resource::<TrainingArena>().is_none());
		Ok(())
	}

	#[test]
	fn presented_hosts_of_the_roster_cell_are_training_brawlers() -> anyhow::Result<()> {
		let mut world = roster_world(TrainingRound::new(7), Vec2::ZERO, Vec2::splat(36.0))?;
		world.init_resource::<Messages<MobCellPresented>>();
		publish_then_write(&mut world)?;
		let mob_id = world.resource::<TrainingRoster>().mob_id;
		let host = world.spawn_empty().id();
		let stranger = world.spawn_empty().id();
		let other = Id::from_cell(Aabb3d::from_min_max(Vec3::splat(9_000.0), Vec3::splat(9_001.0)));
		world
			.run_system_once(move |mut presented: MessageWriter<MobCellPresented>| {
				presented.write(MobCellPresented { id: mob_id, hosts: vec![host] });
				presented.write(MobCellPresented { id: other, hosts: vec![stranger] });
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(tag_training_brawlers)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(world.get::<TrainingBrawler>(host).is_some());
		anyhow::ensure!(world.get::<TrainingBrawler>(stranger).is_none());
		Ok(())
	}
}
